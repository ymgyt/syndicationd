use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite, Transaction};
use synd_feed::{
    entry::EntryId,
    types::{Annotated, Category, FeedUrl, Requirement},
};
use synd_registry::{
    RegistryDbResult,
    db::TimelineDb,
    query::{
        TimelineChange, TimelineChangesPage, TimelineChangesQuery, TimelineEntriesPage,
        TimelineEntriesQuery, TimelineEntry, TimelineEntryCursor,
    },
    subscription::{SubscriberId, SubscriptionKey},
    timeline::TimelineCatchup,
};

use super::{
    codec::{decode_stored_entry, decode_stored_feed_meta},
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    pagination::PageLimit,
};

const TIMELINE_ENTRY_SELECT: &str = r#"
SELECT
    te.order_time,
    te.entry_id,
    e.entry_json,
    f.url AS feed_url,
    fs.meta_json,
    s.requirement,
    s.category
FROM timeline_entry AS te
INNER JOIN entry AS e
    ON e.entry_id = te.entry_id
INNER JOIN feed AS f
    ON f.pk = e.feed_pk
INNER JOIN feed_snapshot AS fs
    ON fs.feed_pk = f.pk
INNER JOIN feed_subscription AS s
    ON s.subscriber_id = te.subscriber_id
   AND s.feed_pk = f.pk
"#;

async fn ensure_timeline(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            INSERT OR IGNORE INTO timeline (subscriber_id)
            VALUES (?)
            "#,
    )
    .bind(subscriber_id.as_str())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn catchup_feed(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
    feed_url: &FeedUrl,
) -> SqliteResult<TimelineCatchup> {
    FeedCatchupCandidates::load(tx, subscriber_id, feed_url)
        .await?
        .apply(tx)
        .await
}

/// Current feed members that may need to be added to one subscriber timeline.
struct FeedCatchupCandidates {
    subscriber_id: SubscriberId,
    feed_url: FeedUrl,
    count: i64,
}

impl FeedCatchupCandidates {
    async fn load(
        tx: &mut Transaction<'_, Sqlite>,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> SqliteResult<Self> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
                SELECT COUNT(*)
                FROM feed_entry AS fe
                INNER JOIN feed AS f
                    ON f.pk = fe.feed_pk
                WHERE f.url = ?
                "#,
        )
        .bind(feed_url.as_str())
        .fetch_one(&mut **tx)
        .await?;
        Ok(Self {
            subscriber_id: subscriber_id.clone(),
            feed_url: feed_url.clone(),
            count,
        })
    }

    async fn apply(self, tx: &mut Transaction<'_, Sqlite>) -> SqliteResult<TimelineCatchup> {
        if self.count == 0 {
            return Ok(self.outcome(0));
        }

        // Every row gets its own seq: change pagination pages by seq alone, so
        // rows sharing a seq would be lost at page boundaries. Candidates left
        // untouched leave gaps, which is fine because seq only needs to be
        // unique and increasing.
        let base_seq = alloc_seq_range(tx, &self.subscriber_id, self.count).await?;
        let inserted = self.insert(tx, base_seq).await?;
        Ok(self.outcome(inserted))
    }

    async fn insert(&self, tx: &mut Transaction<'_, Sqlite>, base_seq: i64) -> SqliteResult<u64> {
        // Insert missing entries and revive tombstoned ones(resubscribe).
        // Live rows are left untouched so they emit no sync change.
        let result = sqlx::query(
            r#"
                INSERT INTO timeline_entry (
                    subscriber_id,
                    entry_id,
                    order_time,
                    seq
                )
                SELECT
                    ?,
                    e.entry_id,
                    e.order_time,
                    ? + ROW_NUMBER() OVER (ORDER BY e.entry_id)
                FROM feed_entry AS fe
                INNER JOIN entry AS e
                    ON e.feed_pk = fe.feed_pk
                   AND e.entry_id = fe.entry_id
                INNER JOIN feed AS f
                    ON f.pk = fe.feed_pk
                WHERE f.url = ?
                ON CONFLICT (subscriber_id, entry_id) DO UPDATE SET
                    seq = excluded.seq,
                    deleted = 0
                WHERE timeline_entry.deleted != 0
                "#,
        )
        .bind(self.subscriber_id.as_str())
        .bind(base_seq)
        .bind(self.feed_url.as_str())
        .execute(&mut **tx)
        .await?;
        Ok(result.rows_affected())
    }

    fn outcome(&self, added: u64) -> TimelineCatchup {
        TimelineCatchup::new(self.subscriber_id.clone(), self.feed_url.clone(), added)
    }
}

async fn apply_entry_to_timelines(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_id: &EntryId,
    content_changed: bool,
) -> SqliteResult<Vec<SubscriberId>> {
    ensure_subscriber_timelines(tx, feed_url).await?;

    let mut affected = Vec::new();
    for target in load_entry_timeline_targets(tx, feed_url, entry_id).await? {
        let touched = match &target.existing {
            None => {
                let seq = next_seq(tx, &target.subscriber_id).await?;
                insert_timeline_entry(tx, &target, seq).await?;
                true
            }
            // order_time is frozen; the seq bump only tells syncing clients
            // to re-read the entry content or that a tombstone was revived
            Some(existing) if content_changed || existing.deleted => {
                let seq = next_seq(tx, &target.subscriber_id).await?;
                bump_timeline_entry(tx, &target, seq).await?;
                true
            }
            Some(_) => false,
        };
        if touched {
            affected.push(target.subscriber_id);
        }
    }
    Ok(affected)
}

async fn apply_feed_unsubscribed(
    tx: &mut Transaction<'_, Sqlite>,
    subscription: &SubscriptionKey,
) -> SqliteResult<Option<SubscriberId>> {
    let subscriber_id = &subscription.subscriber_id;
    if !timeline_exists(tx, subscriber_id).await? {
        return Ok(None);
    }
    let candidates = sqlx::query_scalar::<_, i64>(
        r#"
            SELECT COUNT(*)
            FROM timeline_entry AS te
            WHERE te.subscriber_id = ?
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
    .bind(subscriber_id.as_str())
    .bind(subscription.feed_url.as_str())
    .fetch_one(&mut **tx)
    .await?;
    if candidates == 0 {
        return Ok(None);
    }

    // Removal keeps the row as a tombstone so syncing clients observe it
    // through the seq bump. Every row gets its own seq: change pagination
    // pages by seq alone, so rows sharing a seq would be lost at page
    // boundaries
    let base_seq = alloc_seq_range(tx, subscriber_id, candidates).await?;
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
                WHERE te.subscriber_id = ?
                  AND te.deleted = 0
                  AND te.entry_id IN (
                    SELECT e.entry_id
                    FROM entry AS e
                    INNER JOIN feed AS f
                        ON f.pk = e.feed_pk
                    WHERE f.url = ?
                  )
            ) AS ranked
            WHERE timeline_entry.subscriber_id = ?
              AND timeline_entry.entry_id = ranked.entry_id
            "#,
    )
    .bind(base_seq)
    .bind(subscriber_id.as_str())
    .bind(subscription.feed_url.as_str())
    .bind(subscriber_id.as_str())
    .execute(&mut **tx)
    .await?;

    Ok(Some(subscriber_id.clone()))
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
        let Some(seq) = load_last_seq(tx, &query.subscriber_id).await? else {
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
    sql.push(" WHERE te.subscriber_id = ");
    sql.push_bind(query.subscriber_id.as_str());
    sql.push(" AND te.deleted = 0");
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
        let Some(last_seq) = load_last_seq(tx, &query.subscriber_id).await? else {
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
                s.requirement,
                s.category
            FROM timeline_entry AS te
            INNER JOIN entry AS e
                ON e.entry_id = te.entry_id
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            LEFT JOIN feed_snapshot AS fs
                ON fs.feed_pk = f.pk
            LEFT JOIN feed_subscription AS s
                ON s.subscriber_id = te.subscriber_id
               AND s.feed_pk = f.pk
            WHERE te.subscriber_id = ?
              AND te.seq > ?
            ORDER BY te.seq ASC
            LIMIT ?
            "#,
    )
    .bind(query.subscriber_id.as_str())
    .bind(query.since)
    .bind(limit.sql_limit())
    .fetch_all(&mut **tx)
    .await?;
    Ok(rows)
}

async fn timeline_exists(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
) -> SqliteResult<bool> {
    Ok(load_last_seq(tx, subscriber_id).await?.is_some())
}

async fn load_last_seq(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
) -> SqliteResult<Option<i64>> {
    let row = sqlx::query_scalar::<_, i64>(
        r#"
            SELECT last_seq
            FROM timeline
            WHERE subscriber_id = ?
            "#,
    )
    .bind(subscriber_id.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row)
}

/// Takes the next change seq of one timeline.
async fn next_seq(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
) -> SqliteResult<i64> {
    Ok(alloc_seq_range(tx, subscriber_id, 1).await? + 1)
}

/// Allocates `count` change seqs of one timeline in one contiguous range.
/// The allocated seqs are `base + 1 ..= base + count` where `base` is the
/// returned value.
async fn alloc_seq_range(
    tx: &mut Transaction<'_, Sqlite>,
    subscriber_id: &SubscriberId,
    count: i64,
) -> SqliteResult<i64> {
    let last_seq = sqlx::query_scalar::<_, i64>(
        r#"
            UPDATE timeline
            SET last_seq = last_seq + ?
            WHERE subscriber_id = ?
            RETURNING last_seq
            "#,
    )
    .bind(count)
    .bind(subscriber_id.as_str())
    .fetch_one(&mut **tx)
    .await?;
    Ok(last_seq - count)
}

/// Ensures every subscriber of the feed has a timeline row to allocate
/// seqs from.
async fn ensure_subscriber_timelines(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            INSERT OR IGNORE INTO timeline (subscriber_id)
            SELECT s.subscriber_id
            FROM feed_subscription AS s
            INNER JOIN feed AS f
                ON f.pk = s.feed_pk
            WHERE f.url = ?
            "#,
    )
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
                s.subscriber_id,
                e.entry_id,
                e.order_time AS entry_order_time,
                te.entry_id IS NOT NULL AS entry_exists,
                COALESCE(te.deleted, 0) != 0 AS entry_deleted
            FROM feed_subscription AS s
            INNER JOIN feed AS f
                ON f.pk = s.feed_pk
            INNER JOIN feed_entry AS fe
                ON fe.feed_pk = f.pk
            INNER JOIN entry AS e
                ON e.feed_pk = fe.feed_pk
               AND e.entry_id = fe.entry_id
            LEFT JOIN timeline_entry AS te
                ON te.subscriber_id = s.subscriber_id
               AND te.entry_id = e.entry_id
            WHERE f.url = ?
              AND e.entry_id = ?
            ORDER BY s.subscriber_id
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
                subscriber_id,
                entry_id,
                order_time,
                seq
            )
            VALUES (?, ?, ?, ?)
            "#,
    )
    .bind(target.subscriber_id.as_str())
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
            WHERE subscriber_id = ?
              AND entry_id = ?
            "#,
    )
    .bind(seq)
    .bind(target.subscriber_id.as_str())
    .bind(&target.entry_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

struct TimelineEntryTarget {
    subscriber_id: SubscriberId,
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
    entry_id: String,
    entry_order_time: DateTime<Utc>,
    entry_exists: bool,
    entry_deleted: bool,
}

impl From<TimelineEntryTargetRow> for TimelineEntryTarget {
    fn from(row: TimelineEntryTargetRow) -> Self {
        Self {
            subscriber_id: SubscriberId::new(row.subscriber_id),
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
    requirement: Option<String>,
    category: Option<String>,
}

impl TryFrom<TimelineEntryRow> for TimelineEntry {
    type Error = SqliteError;

    fn try_from(row: TimelineEntryRow) -> Result<Self, Self::Error> {
        let entry = decode_stored_entry(&row.entry.entry_id, &row.entry.entry_json)?;
        let feed_meta = decode_stored_feed_meta(&row.entry.feed_url, &row.meta_json)?;
        let requirement = row
            .entry
            .requirement
            .as_deref()
            .map(Requirement::from_str)
            .transpose()
            .decode()?;
        let category = row.entry.category.map(Category::new).transpose().decode()?;
        let cursor = TimelineEntryCursor::new(row.entry.order_time, entry.id().clone());
        let feed_meta = Annotated {
            feed: feed_meta,
            requirement,
            category,
        };

        Ok(TimelineEntry {
            entry,
            feed_meta,
            cursor,
        })
    }
}

impl TimelineDb for super::SqliteRegistryTx<'_> {
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
        ensure_timeline(&mut self.tx, subscriber_id).await.db()?;
        catchup_feed(&mut self.tx, subscriber_id, feed_url)
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
