use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite, Transaction};
use synd_feed::types::{Annotated, Category, EntryId, FeedMeta, FeedUrl, Requirement};
use synd_registry::{
    RegistryDbResult, TimelineStore,
    entry::EntryAttrs,
    query::{
        TimelineChange, TimelineChangesPage, TimelineChangesQuery, TimelineEntryCursor,
        TimelineEntry, TimelineEntriesPage, TimelineEntriesQuery,
    },
    subscription::{SubscriberId, Subscription, SubscriptionKey},
    timeline::{TimelineCatchup, TimelineKey, TimelineKind},
};

use super::{
    codec,
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    pagination::PageLimit,
};

const TIMELINE_ENTRY_SELECT: &str = r#"
SELECT
    ti.order_time,
    ti.entry_id,
    e.current_content_json,
    fe.url AS feed_url,
    f.current_meta_json,
    s.requirement,
    s.category,
    s.crawl_policy_json,
    s.created_at AS subscription_created_at,
    s.updated_at AS subscription_updated_at
FROM timeline AS t
INNER JOIN timeline_entry AS ti
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
    let seq = next_seq(tx, timeline_pk).await?;
    // Insert missing entries and revive tombstoned ones(resubscribe).
    // Live rows are left untouched so they emit no sync change.
    let result = sqlx::query(
        r#"
            INSERT INTO timeline_entry (
                timeline_pk,
                entry_pk,
                entry_id,
                order_time,
                seq,
                created_at
            )
            SELECT
                ?,
                e.pk,
                e.entry_id,
                e.current_order_time,
                ?,
                ?
            FROM entry AS e
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            INNER JOIN feed_endpoint AS fe
                ON fe.pk = f.feed_endpoint_pk
            WHERE fe.url = ?
            ON CONFLICT (timeline_pk, entry_pk) DO UPDATE SET
                seq = excluded.seq,
                deleted_at = NULL
            WHERE timeline_entry.deleted_at IS NOT NULL
            "#,
    )
    .bind(timeline_pk)
    .bind(seq)
    .bind(now)
    .bind(feed_url.as_str())
    .execute(&mut **tx)
    .await?;

    Ok(TimelineCatchup::new(
        timeline.clone(),
        feed_url.clone(),
        result.rows_affected(),
    ))
}

async fn apply_entry_to_timelines(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_id: &EntryId,
    content_changed: bool,
    now: DateTime<Utc>,
) -> SqliteResult<Vec<TimelineKey>> {
    catchup_subscribed_feeds_for_feed(tx, feed_url, now).await?;

    let mut affected = Vec::new();
    for target in load_entry_timeline_targets(tx, feed_url, entry_id).await? {
        let touched = match &target.existing {
            None => {
                let seq = next_seq(tx, target.timeline_pk).await?;
                insert_timeline_entry(tx, &target, seq, now).await?;
                true
            }
            // order_time is frozen; the seq bump only tells syncing clients
            // to re-read the entry content or that a tombstone was revived
            Some(existing) if content_changed || existing.deleted => {
                let seq = next_seq(tx, target.timeline_pk).await?;
                bump_timeline_entry(tx, &target, seq).await?;
                true
            }
            Some(_) => false,
        };
        if touched {
            affected.push(target.timeline);
        }
    }
    Ok(affected)
}

async fn apply_feed_unsubscribed(
    tx: &mut Transaction<'_, Sqlite>,
    subscription: &SubscriptionKey,
    now: DateTime<Utc>,
) -> SqliteResult<Option<TimelineKey>> {
    let timeline = TimelineKey::default_for(subscription.subscriber_id.clone());
    let Some(head) = try_resolve_timeline(tx, &timeline).await? else {
        return Ok(None);
    };
    let timeline_pk = head.pk;
    let seq = next_seq(tx, timeline_pk).await?;
    // Removal keeps the row as a tombstone so syncing clients observe it
    // through the seq bump
    let result = sqlx::query(
        r#"
            UPDATE timeline_entry
            SET
                deleted_at = ?,
                seq = ?
            WHERE timeline_pk = ?
              AND deleted_at IS NULL
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
    .bind(now)
    .bind(seq)
    .bind(timeline_pk)
    .bind(subscription.feed_url.as_str())
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() == 0 {
        return Ok(None);
    }
    Ok(Some(timeline))
}

async fn list_entries(
    tx: &mut Transaction<'_, Sqlite>,
    query: TimelineEntriesQuery,
) -> SqliteResult<TimelineEntriesPage> {
    let timeline = TimelineKey::default_for(query.subscriber_id.clone());
    let Some(head) = try_resolve_timeline(tx, &timeline).await? else {
        return Ok(TimelineEntriesPage {
            nodes: Vec::new(),
            has_next_page: false,
            end_cursor: None,
            seq: 0,
        });
    };

    let page_limit = PageLimit::new(query.first);
    let mut sql = QueryBuilder::<Sqlite>::new(TIMELINE_ENTRY_SELECT);
    sql.push(" WHERE ti.timeline_pk = ");
    sql.push_bind(head.pk);
    sql.push(" AND ti.deleted_at IS NULL");
    if let Some(feed_url) = query.feed_url.as_ref() {
        sql.push(" AND fe.url = ");
        sql.push_bind(feed_url.as_str());
    }

    if let Some(after) = query.after.as_ref() {
        sql.push(" AND (ti.order_time, ti.entry_id) < (");
        sql.push_bind(after.order_time());
        sql.push(", ");
        sql.push_bind(after.entry_id().as_str());
        sql.push(")");
    }

    sql.push(" ORDER BY ti.order_time DESC, ti.entry_id DESC LIMIT ");
    sql.push_bind(page_limit.sql_limit());

    let rows = sql
        .build_query_as::<TimelineEntryRow>()
        .fetch_all(&mut **tx)
        .await?;
    let mut nodes = rows
        .into_iter()
        .map(|row| row.into_node(&query.subscriber_id))
        .collect::<SqliteResult<Vec<_>>>()?;
    let has_next_page = page_limit.truncate_overfetch(&mut nodes);
    let end_cursor = nodes.last().map(|node| node.cursor.clone());

    Ok(TimelineEntriesPage {
        nodes,
        has_next_page,
        end_cursor,
        seq: head.last_seq,
    })
}

async fn list_changes(
    tx: &mut Transaction<'_, Sqlite>,
    query: TimelineChangesQuery,
) -> SqliteResult<TimelineChangesPage> {
    let timeline = TimelineKey::default_for(query.subscriber_id.clone());
    let Some(head) = try_resolve_timeline(tx, &timeline).await? else {
        return Ok(TimelineChangesPage {
            changes: Vec::new(),
            seq: 0,
            has_more: false,
        });
    };

    let page_limit = PageLimit::new(query.limit);
    // Tombstoned entries have lost their subscription, so subscription columns
    // are joined optionally and their absence also means removal
    let mut rows = sqlx::query_as::<_, TimelineChangeRow>(
        r#"
            SELECT
                ti.order_time,
                ti.entry_id,
                ti.seq,
                (ti.deleted_at IS NOT NULL OR s.subscriber_id IS NULL) AS removed,
                e.current_content_json,
                fe.url AS feed_url,
                f.current_meta_json,
                s.requirement,
                s.category,
                s.crawl_policy_json,
                s.created_at AS subscription_created_at,
                s.updated_at AS subscription_updated_at
            FROM timeline_entry AS ti
            INNER JOIN entry AS e
                ON e.pk = ti.entry_pk
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            INNER JOIN feed_endpoint AS fe
                ON fe.pk = f.feed_endpoint_pk
            LEFT JOIN feed_endpoint_subscription AS s
                ON s.subscriber_id = ?
               AND s.feed_endpoint_pk = fe.pk
            WHERE ti.timeline_pk = ?
              AND ti.seq > ?
            ORDER BY ti.seq ASC
            LIMIT ?
            "#,
    )
    .bind(query.subscriber_id.as_str())
    .bind(head.pk)
    .bind(query.since)
    .bind(page_limit.sql_limit())
    .fetch_all(&mut **tx)
    .await?;

    let has_more = page_limit.truncate_overfetch(&mut rows);
    let seq = if has_more {
        rows.last().map_or(query.since, |row| row.seq)
    } else {
        head.last_seq
    };
    let changes = rows
        .into_iter()
        .map(|row| row.into_change(&query.subscriber_id))
        .collect::<SqliteResult<Vec<_>>>()?;

    Ok(TimelineChangesPage {
        changes,
        seq,
        has_more,
    })
}

async fn resolve_timeline_pk(
    tx: &mut Transaction<'_, Sqlite>,
    timeline: &TimelineKey,
) -> SqliteResult<i64> {
    let Some(head) = try_resolve_timeline(tx, timeline).await? else {
        return Err(SqliteError::not_found(
            "timeline",
            format!(
                "timeline not found: subscriber_id={}, kind={}",
                timeline.subscriber_id, timeline.kind
            ),
        ));
    };
    Ok(head.pk)
}

async fn try_resolve_timeline(
    tx: &mut Transaction<'_, Sqlite>,
    timeline: &TimelineKey,
) -> SqliteResult<Option<TimelineHead>> {
    let row = sqlx::query_as::<_, TimelineHead>(
        r#"
            SELECT pk, last_seq
            FROM timeline
            WHERE subscriber_id = ?
              AND kind = ?
            "#,
    )
    .bind(timeline.subscriber_id.as_str())
    .bind(timeline.kind.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row)
}

/// Identity and current change position of one timeline row.
#[derive(sqlx::FromRow)]
struct TimelineHead {
    pk: i64,
    last_seq: i64,
}

/// Takes the next change seq of one timeline.
async fn next_seq(tx: &mut Transaction<'_, Sqlite>, timeline_pk: i64) -> SqliteResult<i64> {
    let seq = sqlx::query_scalar::<_, i64>(
        r#"
            UPDATE timeline
            SET last_seq = last_seq + 1
            WHERE pk = ?
            RETURNING last_seq
            "#,
    )
    .bind(timeline_pk)
    .fetch_one(&mut **tx)
    .await?;
    Ok(seq)
}

async fn catchup_subscribed_feeds_for_feed(
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
                e.entry_id,
                e.current_order_time AS entry_order_time,
                ti.entry_pk IS NOT NULL AS entry_exists,
                ti.deleted_at IS NOT NULL AS entry_deleted
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
            LEFT JOIN timeline_entry AS ti
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

async fn insert_timeline_entry(
    tx: &mut Transaction<'_, Sqlite>,
    target: &TimelineEntryTarget,
    seq: i64,
    now: DateTime<Utc>,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            INSERT INTO timeline_entry (
                timeline_pk,
                entry_pk,
                entry_id,
                order_time,
                seq,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
    )
    .bind(target.timeline_pk)
    .bind(target.entry_pk)
    .bind(target.entry_id.as_str())
    .bind(target.entry_order_time)
    .bind(seq)
    .bind(now)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn bump_timeline_entry(
    tx: &mut Transaction<'_, Sqlite>,
    target: &TimelineEntryTarget,
    seq: i64,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            UPDATE timeline_entry
            SET
                seq = ?,
                deleted_at = NULL
            WHERE timeline_pk = ?
              AND entry_pk = ?
            "#,
    )
    .bind(seq)
    .bind(target.timeline_pk)
    .bind(target.entry_pk)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

struct TimelineEntryTarget {
    timeline_pk: i64,
    timeline: TimelineKey,
    entry_pk: i64,
    entry_id: String,
    entry_order_time: DateTime<Utc>,
    existing: Option<ExistingTimelineEntry>,
}

struct ExistingTimelineEntry {
    deleted: bool,
}

#[derive(sqlx::FromRow)]
struct TimelineEntryTargetRow {
    timeline_pk: i64,
    subscriber_id: String,
    entry_pk: i64,
    entry_id: String,
    entry_order_time: DateTime<Utc>,
    entry_exists: bool,
    entry_deleted: bool,
}

impl TimelineEntryTargetRow {
    fn into_target(self) -> TimelineEntryTarget {
        TimelineEntryTarget {
            timeline_pk: self.timeline_pk,
            timeline: TimelineKey::default_for(SubscriberId::new(self.subscriber_id)),
            entry_pk: self.entry_pk,
            entry_id: self.entry_id,
            entry_order_time: self.entry_order_time,
            existing: self.entry_exists.then_some(ExistingTimelineEntry {
                deleted: self.entry_deleted,
            }),
        }
    }
}

#[derive(sqlx::FromRow)]
struct TimelineChangeRow {
    order_time: DateTime<Utc>,
    entry_id: String,
    seq: i64,
    removed: bool,
    current_content_json: String,
    feed_url: String,
    current_meta_json: String,
    requirement: Option<String>,
    category: Option<String>,
    crawl_policy_json: Option<String>,
    subscription_created_at: Option<DateTime<Utc>>,
    subscription_updated_at: Option<DateTime<Utc>>,
}

impl TimelineChangeRow {
    fn into_change(self, subscriber_id: &SubscriberId) -> SqliteResult<TimelineChange> {
        if self.removed {
            let entry_id = EntryId::parse(self.entry_id).decode()?;
            return Ok(TimelineChange::Remove { entry_id });
        }

        let missing = |column: &str| {
            SqliteError::decode_message(format!(
                "timeline change row without subscription column: {column}"
            ))
        };
        let row = TimelineEntryRow {
            order_time: self.order_time,
            entry_id: self.entry_id,
            current_content_json: self.current_content_json,
            feed_url: self.feed_url,
            current_meta_json: self.current_meta_json,
            requirement: self.requirement,
            category: self.category,
            crawl_policy_json: self
                .crawl_policy_json
                .ok_or_else(|| missing("crawl_policy_json"))?,
            subscription_created_at: self
                .subscription_created_at
                .ok_or_else(|| missing("subscription_created_at"))?,
            subscription_updated_at: self
                .subscription_updated_at
                .ok_or_else(|| missing("subscription_updated_at"))?,
        };
        Ok(TimelineChange::Upsert(Box::new(
            row.into_node(subscriber_id)?,
        )))
    }
}

#[derive(sqlx::FromRow)]
struct TimelineEntryRow {
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

impl TimelineEntryRow {
    fn into_node(self, subscriber_id: &SubscriberId) -> SqliteResult<TimelineEntry> {
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
        let cursor = TimelineEntryCursor::new(self.order_time, entry_id.clone());
        let feed_meta = Annotated {
            feed: feed_meta,
            requirement,
            category,
        };

        Ok(TimelineEntry {
            entry_id,
            attrs,
            feed_meta,
            subscription,
            cursor,
        })
    }
}

impl TimelineStore for super::SqliteRegistryTx<'_> {
    async fn list_timeline_entries(
        &mut self,
        query: TimelineEntriesQuery,
    ) -> RegistryDbResult<TimelineEntriesPage> {
        list_entries(&mut self.tx, query).await.db()
    }

    async fn list_timeline_changes(
        &mut self,
        query: TimelineChangesQuery,
    ) -> RegistryDbResult<TimelineChangesPage> {
        list_changes(&mut self.tx, query).await.db()
    }

    async fn catchup_subscribed_feed(
        &mut self,
        timeline: &TimelineKey,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<TimelineCatchup> {
        ensure_default(&mut self.tx, timeline, now).await.db()?;
        catchup_feed(&mut self.tx, timeline, feed_url, now)
            .await
            .db()
    }

    async fn apply_entry_to_timelines(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        content_changed: bool,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<Vec<TimelineKey>> {
        apply_entry_to_timelines(&mut self.tx, feed_url, entry_id, content_changed, now)
            .await
            .db()
    }

    async fn apply_feed_unsubscribed(
        &mut self,
        subscription: &SubscriptionKey,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<Option<TimelineKey>> {
        apply_feed_unsubscribed(&mut self.tx, subscription, now)
            .await
            .db()
    }
}

#[cfg(test)]
mod tests;
