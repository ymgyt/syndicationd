use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Row, Sqlite, Transaction};
use synd_feed::types::{Annotated, Category, EntryId, FeedMeta, FeedUrl, Requirement};
use synd_registry::{
    RegistryDbError, RegistryDbResult, TimelineProjectionTx,
    entry::EntryAttrs,
    query::{TimelineItemCursor, TimelineItemNode, TimelineItemsPage, TimelineItemsQuery},
    subscription::{SubscriberId, Subscription},
    timeline::{TimelineCatchup, TimelineKey, TimelineKind},
};

use super::codec;

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

pub(super) struct TimelineTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> TimelineTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn ensure_default(
        &mut self,
        timeline: &TimelineKey,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;
        Ok(())
    }

    pub(super) async fn catchup_feed(
        &mut self,
        timeline: &TimelineKey,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<TimelineCatchup> {
        let timeline_pk = self.resolve_timeline_pk(timeline).await?;
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(TimelineCatchup::new(
            timeline.clone(),
            feed_url.clone(),
            result.rows_affected(),
        ))
    }

    pub(super) async fn list_items(
        &mut self,
        query: TimelineItemsQuery,
    ) -> RegistryDbResult<TimelineItemsPage> {
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
            .build()
            .fetch_all(&mut **self.tx)
            .await
            .map_err(RegistryDbError::internal)?;
        let mut nodes = rows
            .iter()
            .map(|row| timeline_item_node(&query.subscriber_id, row))
            .collect::<RegistryDbResult<Vec<_>>>()?;
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

    async fn resolve_timeline_pk(&mut self, timeline: &TimelineKey) -> RegistryDbResult<i64> {
        let row = sqlx::query(
            r#"
            SELECT pk
            FROM timeline
            WHERE subscriber_id = ?
              AND kind = ?
            "#,
        )
        .bind(timeline.subscriber_id.as_str())
        .bind(timeline.kind.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "timeline not found: subscriber_id={}, kind={}",
                timeline.subscriber_id,
                timeline.kind
            )));
        };
        row.try_get::<i64, _>("pk")
            .map_err(RegistryDbError::internal)
    }
}

impl TimelineProjectionTx for super::SqliteRegistryTx<'_> {
    async fn ensure_default_timeline(
        &mut self,
        timeline: &TimelineKey,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        TimelineTable::new(&mut self.tx)
            .ensure_default(timeline, now)
            .await
    }

    async fn catchup_timeline_feed(
        &mut self,
        timeline: &TimelineKey,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> RegistryDbResult<TimelineCatchup> {
        TimelineTable::new(&mut self.tx)
            .catchup_feed(timeline, feed_url, now)
            .await
    }
}

fn timeline_item_node(
    subscriber_id: &SubscriberId,
    row: &sqlx::sqlite::SqliteRow,
) -> RegistryDbResult<TimelineItemNode> {
    let order_time = row
        .try_get::<DateTime<Utc>, _>("order_time")
        .map_err(RegistryDbError::internal)?;
    let entry_id = EntryId::parse(
        row.try_get::<String, _>("entry_id")
            .map_err(RegistryDbError::internal)?,
    )
    .map_err(RegistryDbError::internal)?;
    let feed_url = FeedUrl::parse(
        &row.try_get::<String, _>("feed_url")
            .map_err(RegistryDbError::internal)?,
    )
    .map_err(RegistryDbError::internal)?;
    let attrs = serde_json::from_str::<EntryAttrs>(
        &row.try_get::<String, _>("current_content_json")
            .map_err(RegistryDbError::internal)?,
    )
    .map_err(RegistryDbError::internal)?;
    let feed_meta = serde_json::from_str::<FeedMeta>(
        &row.try_get::<String, _>("current_meta_json")
            .map_err(RegistryDbError::internal)?,
    )
    .map_err(RegistryDbError::internal)?;
    let requirement = row
        .try_get::<Option<String>, _>("requirement")
        .map_err(RegistryDbError::internal)?
        .as_deref()
        .map(Requirement::from_str)
        .transpose()
        .map_err(|err| RegistryDbError::internal(anyhow::anyhow!(err)))?;
    let category = row
        .try_get::<Option<String>, _>("category")
        .map_err(RegistryDbError::internal)?
        .map(Category::new)
        .transpose()
        .map_err(RegistryDbError::internal)?;
    let crawl_policy = codec::decode_crawl_policy_json(
        &row.try_get::<String, _>("crawl_policy_json")
            .map_err(RegistryDbError::internal)?,
    )?;
    let created_at = row
        .try_get::<DateTime<Utc>, _>("subscription_created_at")
        .map_err(RegistryDbError::internal)?;
    let updated_at = row
        .try_get::<DateTime<Utc>, _>("subscription_updated_at")
        .map_err(RegistryDbError::internal)?;
    let subscription = Subscription {
        subscriber_id: subscriber_id.clone(),
        feed_url: feed_url.clone(),
        requirement,
        category: category.clone(),
        crawl_policy,
        created_at,
        updated_at,
    };
    let cursor = TimelineItemCursor::new(order_time, feed_url, entry_id);
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
