use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite, Transaction};
use synd_feed::{
    entry::EntryId,
    types::{Annotated, FeedUrl, SubscriptionLabels},
};

use super::{
    codec::{decode_stored_entry, decode_stored_feed_meta},
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    pagination::PageLimit,
};
use crate::{
    RegistryDbResult,
    db::{InsertTimelineOutcome, TimelineDb},
    event::TimelineCreatedEvent,
    query::{
        Timeline, TimelineChange, TimelineChangesPage, TimelineChangesQuery, TimelineEntriesPage,
        TimelineEntriesQuery, TimelineEntry, TimelineEntryCursor, TimelinesQuery,
    },
    subscription::{SubscriberId, SubscriptionKey},
    timeline::{TimelineCatchup, TimelineId},
};

const TIMELINE_ENTRY_SELECT: &str = r#"
SELECT
    te.order_time,
    te.entry_id,
    e.entry_json,
    f.url AS feed_url,
    fs.meta_json,
    s.labels_json
FROM timeline_entry AS te
INNER JOIN timeline AS t ON t.timeline_id = te.timeline_id
INNER JOIN entry AS e
    ON e.entry_id = te.entry_id
INNER JOIN feed AS f
    ON f.pk = e.feed_pk
INNER JOIN feed_snapshot AS fs
    ON fs.feed_pk = f.pk
INNER JOIN feed_subscription AS s
    ON s.subscriber_id = t.subscriber_id
   AND s.feed_pk = f.pk
"#;

async fn insert_timeline(
    tx: &mut Transaction<'_, Sqlite>,
    timeline: &TimelineCreatedEvent,
) -> SqliteResult<InsertTimelineOutcome> {
    let result = sqlx::query(
        r#"
        INSERT INTO timeline (timeline_id, subscriber_id, name)
        VALUES (?, ?, ?)
        ON CONFLICT (subscriber_id, name) DO NOTHING
        "#,
    )
    .bind(timeline.timeline_id)
    .bind(timeline.subscriber_id.as_str())
    .bind(&timeline.name)
    .execute(&mut **tx)
    .await?;
    Ok(if result.rows_affected() == 0 {
        InsertTimelineOutcome::NameAlreadyExists
    } else {
        InsertTimelineOutcome::Inserted
    })
}

async fn subscriber_timeline_ids(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
) -> SqliteResult<Vec<TimelineId>> {
    Ok(
        sqlx::query_scalar("SELECT timeline_id FROM timeline WHERE subscriber_id = ?")
            .bind(subscriber_id.as_str())
            .fetch_all(&mut **tx)
            .await?,
    )
}

async fn catchup_created_timeline(
    tx: &mut Transaction<'_, Sqlite>,
    timeline_id: TimelineId,
) -> SqliteResult<Vec<FeedUrl>> {
    let feeds = sqlx::query_scalar::<_, FeedUrl>(
        r#"
        SELECT f.url
        FROM timeline AS t
        JOIN feed_subscription AS s ON s.subscriber_id = t.subscriber_id
        JOIN feed AS f ON f.pk = s.feed_pk
        WHERE t.timeline_id = ?
        ORDER BY f.url
        "#,
    )
    .bind(timeline_id)
    .fetch_all(&mut **tx)
    .await?;
    let mut affected = Vec::new();
    for feed in feeds {
        if catchup_feed(tx, timeline_id, &feed, false).await? > 0 {
            affected.push(feed);
        }
    }
    Ok(affected)
}

async fn catchup_subscribed_feed(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
    feed_url: &FeedUrl,
) -> SqliteResult<TimelineCatchup> {
    let mut inserted = 0;
    for timeline_id in subscriber_timeline_ids(tx, subscriber_id).await? {
        inserted += catchup_feed(tx, timeline_id, feed_url, true).await?;
    }
    Ok(TimelineCatchup::new(
        subscriber_id.clone(),
        feed_url.clone(),
        inserted,
    ))
}

// A new timeline considers the whole entry catalog. A new subscription
// catches up the current feed snapshot into the subscriber's existing timelines.
async fn catchup_feed(
    tx: &mut Transaction<'_, Sqlite>,
    timeline_id: TimelineId,
    feed_url: &FeedUrl,
    current_snapshot_only: bool,
) -> SqliteResult<u64> {
    let inserted = sqlx::query(
        r#"
        INSERT INTO timeline_entry (timeline_id, entry_id, order_time, seq)
        SELECT t.timeline_id, e.entry_id, e.order_time,
               t.last_seq + ROW_NUMBER() OVER (ORDER BY e.entry_id)
        FROM timeline AS t
        JOIN feed_subscription AS s ON s.subscriber_id = t.subscriber_id
        JOIN feed AS f ON f.pk = s.feed_pk
        JOIN entry AS e ON e.feed_pk = f.pk
        LEFT JOIN timeline_entry AS te ON te.timeline_id = t.timeline_id AND te.entry_id = e.entry_id
        WHERE t.timeline_id = ? AND f.url = ?
          AND (te.entry_id IS NULL OR te.deleted != 0)
          AND (NOT ? OR EXISTS (
              SELECT 1 FROM feed_entry AS fe WHERE fe.feed_pk = f.pk AND fe.entry_id = e.entry_id
          ))
        ON CONFLICT (timeline_id, entry_id) DO UPDATE SET seq = excluded.seq, deleted = 0
        "#,
    )
    .bind(timeline_id)
    .bind(feed_url.as_str())
    .bind(current_snapshot_only)
    .execute(&mut **tx)
    .await?
    .rows_affected();
    if inserted > 0 {
        alloc_seq_range(tx, timeline_id, i64::try_from(inserted).decode()?).await?;
    }
    Ok(inserted)
}

async fn apply_entry_to_timelines(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_id: &EntryId,
    content_changed: bool,
) -> SqliteResult<Vec<SubscriberId>> {
    let mut affected = Vec::new();
    for target in load_entry_timeline_targets(tx, feed_url, entry_id).await? {
        let touched = match &target.existing {
            None => {
                let seq = next_seq(tx, target.timeline_id).await?;
                insert_timeline_entry(tx, &target, seq).await?;
                true
            }
            // order_time is frozen; the seq bump only tells syncing clients
            // to re-read the entry content or that a tombstone was revived
            Some(existing) if content_changed || existing.deleted => {
                let seq = next_seq(tx, target.timeline_id).await?;
                bump_timeline_entry(tx, &target, seq).await?;
                true
            }
            Some(_) => false,
        };
        if touched {
            affected.push(target.subscriber_id);
        }
    }
    affected.dedup();
    Ok(affected)
}

async fn apply_feed_unsubscribed(
    tx: &mut Transaction<'_, Sqlite>,
    subscription: &SubscriptionKey,
) -> SqliteResult<Option<SubscriberId>> {
    let subscriber_id = &subscription.subscriber_id;
    let mut changed = false;
    for timeline_id in subscriber_timeline_ids(tx, subscriber_id).await? {
        changed |= remove_feed_entries(tx, timeline_id, &subscription.feed_url).await?;
    }
    Ok(changed.then(|| subscriber_id.clone()))
}

async fn remove_feed_entries(
    tx: &mut Transaction<'_, Sqlite>,
    timeline_id: TimelineId,
    feed_url: &FeedUrl,
) -> SqliteResult<bool> {
    let candidates = sqlx::query_scalar::<_, i64>(
        r#"
            SELECT COUNT(*)
            FROM timeline_entry AS te
            WHERE te.timeline_id = ?
              AND te.deleted = 0
              AND te.entry_id IN (
                SELECT e.entry_id
                FROM entry AS e
                INNER JOIN feed AS f
                    ON f.pk = e.feed_pk
                WHERE f.url = ?
              )
            "#,
    )
    .bind(timeline_id)
    .bind(feed_url.as_str())
    .fetch_one(&mut **tx)
    .await?;
    if candidates == 0 {
        return Ok(false);
    }

    // Removal keeps the row as a tombstone so syncing clients observe it
    // through the seq bump. Every row gets its own seq: change pagination
    // pages by seq alone, so rows sharing a seq would be lost at page
    // boundaries
    let base_seq = alloc_seq_range(tx, timeline_id, candidates).await?;
    sqlx::query(
        r#"
            UPDATE timeline_entry
            SET
                deleted = 1,
                seq = ? + ranked.rn
            FROM (
                SELECT
                    te.entry_id,
                    ROW_NUMBER() OVER (ORDER BY te.entry_id) AS rn
                FROM timeline_entry AS te
                WHERE te.timeline_id = ?
                  AND te.deleted = 0
                  AND te.entry_id IN (
                    SELECT e.entry_id
                    FROM entry AS e
                    INNER JOIN feed AS f
                        ON f.pk = e.feed_pk
                    WHERE f.url = ?
                  )
            ) AS ranked
            WHERE timeline_entry.timeline_id = ?
              AND timeline_entry.entry_id = ranked.entry_id
            "#,
    )
    .bind(base_seq)
    .bind(timeline_id)
    .bind(feed_url.as_str())
    .bind(timeline_id)
    .execute(&mut **tx)
    .await?;

    Ok(true)
}

async fn list_entries(
    tx: &mut Transaction<'_, Sqlite>,
    query: TimelineEntriesQuery,
) -> SqliteResult<TimelineEntriesPage> {
    Ok(StoredTimelineEntriesPage::load(tx, query)
        .await?
        .into_page())
}

/// Overfetched timeline entries paired with the snapshot sequence they represent.
struct StoredTimelineEntriesPage {
    nodes: Vec<TimelineEntry>,
    limit: PageLimit,
    seq: i64,
}

impl StoredTimelineEntriesPage {
    async fn load(
        tx: &mut Transaction<'_, Sqlite>,
        query: TimelineEntriesQuery,
    ) -> SqliteResult<Self> {
        let limit = PageLimit::new(query.first);
        let Some(seq) = load_last_seq(tx, &query.subscriber_id, query.timeline_id).await? else {
            return Ok(Self {
                nodes: Vec::new(),
                limit,
                seq: 0,
            });
        };
        let nodes = load_timeline_entries(tx, &query, limit).await?;
        Ok(Self { nodes, limit, seq })
    }

    fn into_page(mut self) -> TimelineEntriesPage {
        let has_next_page = self.limit.truncate_overfetch(&mut self.nodes);
        let end_cursor = self.nodes.last().map(|node| node.cursor.clone());
        TimelineEntriesPage {
            nodes: self.nodes,
            has_next_page,
            end_cursor,
            seq: self.seq,
        }
    }
}

async fn load_timeline_entries(
    tx: &mut Transaction<'_, Sqlite>,
    query: &TimelineEntriesQuery,
    limit: PageLimit,
) -> SqliteResult<Vec<TimelineEntry>> {
    let mut sql = QueryBuilder::<Sqlite>::new(TIMELINE_ENTRY_SELECT);
    sql.push(" WHERE te.timeline_id = ");
    sql.push_bind(query.timeline_id);
    sql.push(" AND t.subscriber_id = ");
    sql.push_bind(query.subscriber_id.as_str());
    sql.push(" AND te.deleted = 0");
    if let Some(since) = query.since {
        sql.push(" AND te.order_time >= ").push_bind(since);
    }
    if let Some(until) = query.until {
        sql.push(" AND te.order_time < ").push_bind(until);
    }
    if let Some(after) = query.after.as_ref() {
        sql.push(" AND (te.order_time, te.entry_id) < (");
        sql.push_bind(after.order_time());
        sql.push(", ");
        sql.push_bind(after.entry_id().as_str());
        sql.push(")");
    }

    sql.push(" ORDER BY te.order_time DESC, te.entry_id DESC LIMIT ");
    sql.push_bind(limit.sql_limit());

    let rows = sql
        .build_query_as::<TimelineEntryRow>()
        .fetch_all(&mut **tx)
        .await?;
    rows.into_iter().map(TimelineEntry::try_from).collect()
}

async fn list_changes(
    tx: &mut Transaction<'_, Sqlite>,
    query: TimelineChangesQuery,
) -> SqliteResult<TimelineChangesPage> {
    StoredTimelineChangesPage::load(tx, query)
        .await?
        .into_page()
}

/// Overfetched timeline change rows paired with their sequence window.
struct StoredTimelineChangesPage {
    rows: Vec<TimelineChangeRow>,
    limit: PageLimit,
    since: i64,
    last_seq: i64,
}

impl StoredTimelineChangesPage {
    async fn load(
        tx: &mut Transaction<'_, Sqlite>,
        query: TimelineChangesQuery,
    ) -> SqliteResult<Self> {
        let limit = PageLimit::new(query.limit);
        let Some(last_seq) = load_last_seq(tx, &query.subscriber_id, query.timeline_id).await?
        else {
            return Ok(Self {
                rows: Vec::new(),
                limit,
                since: query.since,
                last_seq: 0,
            });
        };
        let rows = load_timeline_change_rows(tx, &query, limit).await?;
        Ok(Self {
            rows,
            limit,
            since: query.since,
            last_seq,
        })
    }

    fn into_page(mut self) -> SqliteResult<TimelineChangesPage> {
        let has_more = self.limit.truncate_overfetch(&mut self.rows);
        let seq = if has_more {
            self.rows.last().map_or(self.since, |row| row.seq)
        } else {
            self.last_seq
        };
        let changes = self
            .rows
            .into_iter()
            .map(TimelineChange::try_from)
            .collect::<SqliteResult<Vec<_>>>()?;
        Ok(TimelineChangesPage {
            changes,
            seq,
            has_more,
        })
    }
}

async fn load_timeline_change_rows(
    tx: &mut Transaction<'_, Sqlite>,
    query: &TimelineChangesQuery,
    limit: PageLimit,
) -> SqliteResult<Vec<TimelineChangeRow>> {
    // Tombstoned entries have lost their subscription, so subscription and
    // snapshot columns are joined optionally and their absence also means
    // removal
    let rows = sqlx::query_as::<_, TimelineChangeRow>(
        r#"
            SELECT
                te.order_time,
                te.entry_id,
                te.seq,
                (te.deleted != 0 OR s.subscriber_id IS NULL) AS removed,
                e.entry_json,
                f.url AS feed_url,
                fs.meta_json,
                COALESCE(s.labels_json, '{}') AS labels_json
            FROM timeline_entry AS te
            INNER JOIN timeline AS t ON t.timeline_id = te.timeline_id
            INNER JOIN entry AS e
                ON e.entry_id = te.entry_id
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            LEFT JOIN feed_snapshot AS fs
                ON fs.feed_pk = f.pk
            LEFT JOIN feed_subscription AS s
                ON s.subscriber_id = t.subscriber_id
               AND s.feed_pk = f.pk
            WHERE te.timeline_id = ? AND t.subscriber_id = ?
              AND te.seq > ?
            ORDER BY te.seq ASC
            LIMIT ?
            "#,
    )
    .bind(query.timeline_id)
    .bind(query.subscriber_id.as_str())
    .bind(query.since)
    .bind(limit.sql_limit())
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

async fn load_last_seq(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
    timeline_id: TimelineId,
) -> SqliteResult<Option<i64>> {
    let row = sqlx::query_scalar::<_, i64>(
        r#"
            SELECT last_seq
            FROM timeline
            WHERE timeline_id = ? AND subscriber_id = ?
            "#,
    )
    .bind(timeline_id)
    .bind(subscriber_id.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row)
}

/// Takes the next change seq of one timeline.
async fn next_seq(tx: &mut Transaction<'_, Sqlite>, timeline_id: TimelineId) -> SqliteResult<i64> {
    Ok(alloc_seq_range(tx, timeline_id, 1).await? + 1)
}

/// Allocates `count` change seqs of one timeline in one contiguous range.
/// The allocated seqs are `base + 1 ..= base + count` where `base` is the
/// returned value.
async fn alloc_seq_range(
    tx: &mut Transaction<'_, Sqlite>,
    timeline_id: TimelineId,
    count: i64,
) -> SqliteResult<i64> {
    let last_seq = sqlx::query_scalar::<_, i64>(
        r#"
            UPDATE timeline
            SET last_seq = last_seq + ?
            WHERE timeline_id = ?
            RETURNING last_seq
            "#,
    )
    .bind(count)
    .bind(timeline_id)
    .fetch_one(&mut **tx)
    .await?;
    Ok(last_seq - count)
}

async fn load_entry_timeline_targets(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_id: &EntryId,
) -> SqliteResult<Vec<TimelineEntryTarget>> {
    let rows = sqlx::query_as::<_, TimelineEntryTargetRow>(
        r#"
            SELECT
                s.subscriber_id,
                t.timeline_id,
                e.entry_id,
                e.order_time AS entry_order_time,
                te.entry_id IS NOT NULL AS entry_exists,
                COALESCE(te.deleted, 0) != 0 AS entry_deleted
            FROM feed_subscription AS s
            INNER JOIN timeline AS t ON t.subscriber_id = s.subscriber_id
            INNER JOIN feed AS f
                ON f.pk = s.feed_pk
            INNER JOIN feed_entry AS fe
                ON fe.feed_pk = f.pk
            INNER JOIN entry AS e
                ON e.feed_pk = fe.feed_pk
               AND e.entry_id = fe.entry_id
            LEFT JOIN timeline_entry AS te
                ON te.timeline_id = t.timeline_id
               AND te.entry_id = e.entry_id
            WHERE f.url = ?
              AND e.entry_id = ?
            ORDER BY s.subscriber_id, t.timeline_id
            "#,
    )
    .bind(feed_url.as_str())
    .bind(entry_id.as_str())
    .fetch_all(&mut **tx)
    .await?;

    Ok(rows.into_iter().map(TimelineEntryTarget::from).collect())
}

async fn insert_timeline_entry(
    tx: &mut Transaction<'_, Sqlite>,
    target: &TimelineEntryTarget,
    seq: i64,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            INSERT INTO timeline_entry (
                timeline_id,
                entry_id,
                order_time,
                seq
            )
            VALUES (?, ?, ?, ?)
            "#,
    )
    .bind(target.timeline_id)
    .bind(&target.entry_id)
    .bind(target.entry_order_time)
    .bind(seq)
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
                deleted = 0
            WHERE timeline_id = ?
              AND entry_id = ?
            "#,
    )
    .bind(seq)
    .bind(target.timeline_id)
    .bind(&target.entry_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

struct TimelineEntryTarget {
    subscriber_id: SubscriberId,
    timeline_id: TimelineId,
    entry_id: String,
    entry_order_time: DateTime<Utc>,
    existing: Option<ExistingTimelineEntry>,
}

struct ExistingTimelineEntry {
    deleted: bool,
}

#[derive(sqlx::FromRow)]
struct TimelineEntryTargetRow {
    subscriber_id: String,
    timeline_id: TimelineId,
    entry_id: String,
    entry_order_time: DateTime<Utc>,
    entry_exists: bool,
    entry_deleted: bool,
}

impl From<TimelineEntryTargetRow> for TimelineEntryTarget {
    fn from(row: TimelineEntryTargetRow) -> Self {
        Self {
            subscriber_id: SubscriberId::new(row.subscriber_id),
            timeline_id: row.timeline_id,
            entry_id: row.entry_id,
            entry_order_time: row.entry_order_time,
            existing: row.entry_exists.then_some(ExistingTimelineEntry {
                deleted: row.entry_deleted,
            }),
        }
    }
}

#[derive(sqlx::FromRow)]
struct TimelineChangeRow {
    #[sqlx(flatten)]
    entry: TimelineEntryColumns,
    seq: i64,
    removed: bool,
    meta_json: Option<String>,
}

impl TryFrom<TimelineChangeRow> for TimelineChange {
    type Error = SqliteError;

    fn try_from(row: TimelineChangeRow) -> Result<Self, Self::Error> {
        if row.removed {
            let entry_id = EntryId::parse(row.entry.entry_id).decode()?;
            return Ok(TimelineChange::Remove { entry_id });
        }

        let meta_json = row.meta_json.ok_or_else(|| {
            SqliteError::decode_message("timeline change row without feed snapshot")
        })?;
        let entry = TimelineEntry::try_from(TimelineEntryRow {
            entry: row.entry,
            meta_json,
        })?;
        Ok(TimelineChange::Upsert(Box::new(entry)))
    }
}

#[derive(sqlx::FromRow)]
struct TimelineEntryRow {
    #[sqlx(flatten)]
    entry: TimelineEntryColumns,
    meta_json: String,
}

#[derive(sqlx::FromRow)]
struct TimelineEntryColumns {
    order_time: DateTime<Utc>,
    entry_id: String,
    entry_json: String,
    feed_url: String,
    labels_json: String,
}

impl TryFrom<TimelineEntryRow> for TimelineEntry {
    type Error = SqliteError;

    fn try_from(row: TimelineEntryRow) -> Result<Self, Self::Error> {
        let entry = decode_stored_entry(&row.entry.entry_id, &row.entry.entry_json)?;
        let feed_meta = decode_stored_feed_meta(&row.entry.feed_url, &row.meta_json)?;
        let labels: SubscriptionLabels = serde_json::from_str(&row.entry.labels_json)?;
        let cursor = TimelineEntryCursor::new(row.entry.order_time, entry.id().clone());
        let feed_meta = Annotated {
            feed: feed_meta,
            requirement: labels.requirement(),
            category: labels.category(),
        };

        Ok(TimelineEntry {
            entry,
            feed_meta,
            cursor,
        })
    }
}

impl TimelineDb for super::SqliteRegistryTx<'_> {
    async fn list_timelines(&mut self, query: TimelinesQuery) -> RegistryDbResult<Vec<Timeline>> {
        sqlx::query_as(
            "SELECT timeline_id AS id, name FROM timeline WHERE subscriber_id = ? ORDER BY name",
        )
        .bind(query.subscriber_id.as_str())
        .fetch_all(&mut *self.tx)
        .await
        .map_err(SqliteError::from)
        .db()
    }

    async fn insert_timeline(
        &mut self,
        timeline: &TimelineCreatedEvent,
    ) -> RegistryDbResult<InsertTimelineOutcome> {
        insert_timeline(&mut self.tx, timeline).await.db()
    }

    async fn catchup_created_timeline(
        &mut self,
        timeline_id: TimelineId,
    ) -> RegistryDbResult<Vec<FeedUrl>> {
        catchup_created_timeline(&mut self.tx, timeline_id)
            .await
            .db()
    }

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
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<TimelineCatchup> {
        catchup_subscribed_feed(&mut self.tx, subscriber_id, feed_url)
            .await
            .db()
    }

    async fn apply_entry_to_timelines(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        content_changed: bool,
    ) -> RegistryDbResult<Vec<SubscriberId>> {
        apply_entry_to_timelines(&mut self.tx, feed_url, entry_id, content_changed)
            .await
            .db()
    }

    async fn apply_feed_unsubscribed(
        &mut self,
        subscription: &SubscriptionKey,
    ) -> RegistryDbResult<Option<SubscriberId>> {
        apply_feed_unsubscribed(&mut self.tx, subscription)
            .await
            .db()
    }
}

#[cfg(test)]
mod tests;
