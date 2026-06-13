use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite, Transaction};
use synd_feed::types::{Annotated, Category, EntryId, FeedMeta, FeedUrl, Requirement};
use synd_registry::{
    RegistryDbResult, TimelineTx,
    entry::EntryAttrs,
    query::{TimelineItemCursor, TimelineItemNode, TimelineItemsPage, TimelineItemsQuery},
    subscription::{SubscriberId, Subscription, SubscriptionKey},
    timeline::{TimelineCatchup, TimelineKey, TimelineKind},
};

use super::{
    codec,
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
};

const TIMELINE_ITEM_SELECT: &str = r#"
SELECT
    ti.order_time,
    e.entry_id,
    e.current_content_json,
    fe.url AS feed_url,
    f.current_meta_json,
    s.requirement,
    s.category,
    s.crawl_policy_json,
    s.created_at AS subscription_created_at,
    s.updated_at AS subscription_updated_at
FROM timeline AS t
INNER JOIN timeline_item AS ti
    ON ti.timeline_pk = t.pk
INNER JOIN entry AS e
    ON e.pk = ti.entry_pk
INNER JOIN feed AS f
    ON f.pk = e.feed_pk
INNER JOIN feed_endpoint AS fe
    ON fe.pk = f.feed_endpoint_pk
INNER JOIN feed_endpoint_subscription AS s
    ON s.subscriber_id = t.subscriber_id
   AND s.feed_endpoint_pk = fe.pk
"#;

async fn ensure_default(
    tx: &mut Transaction<'_, Sqlite>,
    timeline: &TimelineKey,
    now: DateTime<Utc>,
) -> SqliteResult<()> {
    debug_assert_eq!(timeline.kind, TimelineKind::Default);
    sqlx::query(
        r#"
            INSERT OR IGNORE INTO timeline (
                subscriber_id,
                kind,
                name,
                definition_json,
                created_at,
                updated_at
            )
            VALUES (?, ?, NULL, NULL, ?, ?)
            "#,
    )
    .bind(timeline.subscriber_id.as_str())
    .bind(timeline.kind.as_str())
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn catchup_feed(
    tx: &mut Transaction<'_, Sqlite>,
    timeline: &TimelineKey,
    feed_url: &FeedUrl,
    now: DateTime<Utc>,
) -> SqliteResult<TimelineCatchup> {
    let timeline_pk = resolve_timeline_pk(tx, timeline).await?;
    let result = sqlx::query(
        r#"
            INSERT INTO timeline_item (
                timeline_pk,
                entry_pk,
                order_time,
                created_at,
                updated_at
            )
            SELECT
                ?,
                e.pk,
                e.current_order_time,
                ?,
                ?
            FROM entry AS e
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            INNER JOIN feed_endpoint AS fe
                ON fe.pk = f.feed_endpoint_pk
            WHERE fe.url = ?
              AND NOT EXISTS (
                SELECT 1
                FROM timeline_item AS ti
                WHERE ti.timeline_pk = ?
                  AND ti.entry_pk = e.pk
              )
            "#,
    )
    .bind(timeline_pk)
    .bind(now)
    .bind(now)
    .bind(feed_url.as_str())
    .bind(timeline_pk)
    .execute(&mut **tx)
    .await?;

    Ok(TimelineCatchup::new(
        timeline.clone(),
        feed_url.clone(),
        result.rows_affected(),
    ))
}

async fn apply_entry_discovered(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_id: &EntryId,
    now: DateTime<Utc>,
) -> SqliteResult<Vec<TimelineKey>> {
    ensure_default_timelines_for_feed(tx, feed_url, now).await?;

    let mut affected = Vec::new();
    for target in load_entry_timeline_targets(tx, feed_url, entry_id).await? {
        if target.item_order_time.is_some() {
            continue;
        }
        insert_entry_item(tx, &target, now).await?;
        affected.push(target.timeline);
    }
    Ok(affected)
}

async fn apply_entry_changed(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_id: &EntryId,
    now: DateTime<Utc>,
) -> SqliteResult<Vec<TimelineKey>> {
    let mut affected = Vec::new();
    for target in load_entry_timeline_targets(tx, feed_url, entry_id).await? {
        let Some(item_order_time) = target.item_order_time else {
            continue;
        };
        if item_order_time != target.entry_order_time {
            update_entry_item_order(tx, &target, now).await?;
        }
        affected.push(target.timeline);
    }
    Ok(affected)
}

async fn apply_feed_unsubscribed(
    tx: &mut Transaction<'_, Sqlite>,
    subscription: &SubscriptionKey,
) -> SqliteResult<Option<TimelineKey>> {
    let result = sqlx::query(
        r#"
            DELETE FROM timeline_item
            WHERE timeline_pk = (
                SELECT t.pk
                FROM timeline AS t
                WHERE t.subscriber_id = ?
                  AND t.kind = ?
            )
              AND entry_pk IN (
                SELECT e.pk
                FROM entry AS e
                INNER JOIN feed AS f
                    ON f.pk = e.feed_pk
                INNER JOIN feed_endpoint AS fe
                    ON fe.pk = f.feed_endpoint_pk
                WHERE fe.url = ?
              )
            "#,
    )
    .bind(subscription.subscriber_id.as_str())
    .bind(TimelineKind::Default.as_str())
    .bind(subscription.feed_url.as_str())
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }
    Ok(Some(TimelineKey::default_for(
        subscription.subscriber_id.clone(),
    )))
}

async fn list_items(
    tx: &mut Transaction<'_, Sqlite>,
    query: TimelineItemsQuery,
) -> SqliteResult<TimelineItemsPage> {
    let first = i64::try_from(query.first.saturating_add(1)).unwrap_or(i64::MAX);
    let mut sql = QueryBuilder::<Sqlite>::new(TIMELINE_ITEM_SELECT);
    sql.push(" WHERE t.subscriber_id = ");
    sql.push_bind(query.subscriber_id.as_str());
    sql.push(" AND t.kind = ");
    sql.push_bind(TimelineKind::Default.as_str());

    if let Some(after) = query.after.as_ref() {
        sql.push(" AND (ti.order_time, fe.url, e.entry_id) < (");
        sql.push_bind(after.order_time());
        sql.push(", ");
        sql.push_bind(after.feed_url().as_str());
        sql.push(", ");
        sql.push_bind(after.entry_id().as_str());
        sql.push(")");
    }

    sql.push(" ORDER BY ti.order_time DESC, fe.url DESC, e.entry_id DESC LIMIT ");
    sql.push_bind(first);

    let rows = sql
        .build_query_as::<TimelineItemRow>()
        .fetch_all(&mut **tx)
        .await?;
    let mut nodes = rows
        .into_iter()
        .map(|row| row.into_node(&query.subscriber_id))
        .collect::<SqliteResult<Vec<_>>>()?;
    let has_next_page = nodes.len() > query.first;
    if has_next_page {
        nodes.truncate(query.first);
    }
    let end_cursor = nodes.last().map(|node| node.cursor.clone());

    Ok(TimelineItemsPage {
        nodes,
        has_next_page,
        end_cursor,
    })
}

async fn resolve_timeline_pk(
    tx: &mut Transaction<'_, Sqlite>,
    timeline: &TimelineKey,
) -> SqliteResult<i64> {
    let row = sqlx::query_as::<_, PkRow>(
        r#"
            SELECT pk
            FROM timeline
            WHERE subscriber_id = ?
              AND kind = ?
            "#,
    )
    .bind(timeline.subscriber_id.as_str())
    .bind(timeline.kind.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Err(SqliteError::not_found(
            "timeline",
            format!(
                "timeline not found: subscriber_id={}, kind={}",
                timeline.subscriber_id, timeline.kind
            ),
        ));
    };
    Ok(row.pk)
}

async fn ensure_default_timelines_for_feed(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    now: DateTime<Utc>,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            INSERT OR IGNORE INTO timeline (
                subscriber_id,
                kind,
                name,
                definition_json,
                created_at,
                updated_at
            )
            SELECT
                s.subscriber_id,
                ?,
                NULL,
                NULL,
                ?,
                ?
            FROM feed_endpoint_subscription AS s
            INNER JOIN feed_endpoint AS fe
                ON fe.pk = s.feed_endpoint_pk
            WHERE fe.url = ?
            "#,
    )
    .bind(TimelineKind::Default.as_str())
    .bind(now)
    .bind(now)
    .bind(feed_url.as_str())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_entry_timeline_targets(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_id: &EntryId,
) -> SqliteResult<Vec<TimelineEntryTarget>> {
    let rows = sqlx::query_as::<_, TimelineEntryTargetRow>(
        r#"
            SELECT
                t.pk AS timeline_pk,
                t.subscriber_id,
                e.pk AS entry_pk,
                e.current_order_time AS entry_order_time,
                ti.order_time AS item_order_time
            FROM feed_endpoint_subscription AS s
            INNER JOIN feed_endpoint AS fe
                ON fe.pk = s.feed_endpoint_pk
            INNER JOIN timeline AS t
                ON t.subscriber_id = s.subscriber_id
               AND t.kind = ?
            INNER JOIN feed AS f
                ON f.feed_endpoint_pk = fe.pk
            INNER JOIN entry AS e
                ON e.feed_pk = f.pk
            LEFT JOIN timeline_item AS ti
                ON ti.timeline_pk = t.pk
               AND ti.entry_pk = e.pk
            WHERE fe.url = ?
              AND e.entry_id = ?
            ORDER BY t.subscriber_id
            "#,
    )
    .bind(TimelineKind::Default.as_str())
    .bind(feed_url.as_str())
    .bind(entry_id.as_str())
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows
        .into_iter()
        .map(TimelineEntryTargetRow::into_target)
        .collect())
}

async fn insert_entry_item(
    tx: &mut Transaction<'_, Sqlite>,
    target: &TimelineEntryTarget,
    now: DateTime<Utc>,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            INSERT INTO timeline_item (
                timeline_pk,
                entry_pk,
                order_time,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?)
            "#,
    )
    .bind(target.timeline_pk)
    .bind(target.entry_pk)
    .bind(target.entry_order_time)
    .bind(now)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn update_entry_item_order(
    tx: &mut Transaction<'_, Sqlite>,
    target: &TimelineEntryTarget,
    now: DateTime<Utc>,
) -> SqliteResult<()> {
    let result = sqlx::query(
        r#"
            UPDATE timeline_item
            SET
                order_time = ?,
                updated_at = ?
            WHERE timeline_pk = ?
              AND entry_pk = ?
            "#,
    )
    .bind(target.entry_order_time)
    .bind(now)
    .bind(target.timeline_pk)
    .bind(target.entry_pk)
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() != 1 {
        return Err(SqliteError::decode_message(format!(
            "timeline item order update affected {} rows for timeline_pk={}, entry_pk={}",
            result.rows_affected(),
            target.timeline_pk,
            target.entry_pk
        )));
    }
    Ok(())
}

struct TimelineEntryTarget {
    timeline_pk: i64,
    timeline: TimelineKey,
    entry_pk: i64,
    entry_order_time: DateTime<Utc>,
    item_order_time: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct TimelineEntryTargetRow {
    timeline_pk: i64,
    subscriber_id: String,
    entry_pk: i64,
    entry_order_time: DateTime<Utc>,
    item_order_time: Option<DateTime<Utc>>,
}

impl TimelineEntryTargetRow {
    fn into_target(self) -> TimelineEntryTarget {
        TimelineEntryTarget {
            timeline_pk: self.timeline_pk,
            timeline: TimelineKey::default_for(SubscriberId::new(self.subscriber_id)),
            entry_pk: self.entry_pk,
            entry_order_time: self.entry_order_time,
            item_order_time: self.item_order_time,
        }
    }
}

#[derive(sqlx::FromRow)]
struct TimelineItemRow {
    order_time: DateTime<Utc>,
    entry_id: String,
    current_content_json: String,
    feed_url: String,
    current_meta_json: String,
    requirement: Option<String>,
    category: Option<String>,
    crawl_policy_json: String,
    subscription_created_at: DateTime<Utc>,
    subscription_updated_at: DateTime<Utc>,
}

impl TimelineItemRow {
    fn into_node(self, subscriber_id: &SubscriberId) -> SqliteResult<TimelineItemNode> {
        let entry_id = EntryId::parse(self.entry_id).decode()?;
        let feed_url = FeedUrl::parse(&self.feed_url).decode()?;
        let attrs = serde_json::from_str::<EntryAttrs>(&self.current_content_json)?;
        let feed_meta = serde_json::from_str::<FeedMeta>(&self.current_meta_json)?;
        let requirement = self
            .requirement
            .as_deref()
            .map(Requirement::from_str)
            .transpose()
            .decode()?;
        let category = self.category.map(Category::new).transpose().decode()?;
        let crawl_policy = codec::decode_crawl_policy_json(&self.crawl_policy_json)?;
        let subscription = Subscription {
            subscriber_id: subscriber_id.clone(),
            feed_url: feed_url.clone(),
            requirement,
            category: category.clone(),
            crawl_policy,
            created_at: self.subscription_created_at,
            updated_at: self.subscription_updated_at,
        };
        let cursor = TimelineItemCursor::new(self.order_time, feed_url, entry_id);
        let feed_meta = Annotated {
            feed: feed_meta,
            requirement,
            category,
        };

        Ok(TimelineItemNode {
            attrs,
            feed_meta,
            subscription,
            cursor,
        })
    }
}

#[derive(sqlx::FromRow)]
struct PkRow {
    pk: i64,
}

impl TimelineTx for super::SqliteRegistryTx<'_> {
    async fn list_timeline_items(
        &mut self,
        query: TimelineItemsQuery,
    ) -> RegistryDbResult<TimelineItemsPage> {
        list_items(&mut self.tx, query).await.db()
    }

    async fn ensure_default_timeline(
        &mut self,
        timeline: &TimelineKey,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        ensure_default(&mut self.tx, timeline, now).await.db()
    }

    async fn catchup_timeline_feed(
        &mut self,
        timeline: &TimelineKey,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<TimelineCatchup> {
        catchup_feed(&mut self.tx, timeline, feed_url, now)
            .await
            .db()
    }

    async fn apply_entry_discovered(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<Vec<TimelineKey>> {
        apply_entry_discovered(&mut self.tx, feed_url, entry_id, now)
            .await
            .db()
    }

    async fn apply_entry_changed(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<Vec<TimelineKey>> {
        apply_entry_changed(&mut self.tx, feed_url, entry_id, now)
            .await
            .db()
    }

    async fn apply_feed_unsubscribed(
        &mut self,
        subscription: &SubscriptionKey,
    ) -> RegistryDbResult<Option<TimelineKey>> {
        apply_feed_unsubscribed(&mut self.tx, subscription)
            .await
            .db()
    }
}

#[cfg(test)]
mod tests;
