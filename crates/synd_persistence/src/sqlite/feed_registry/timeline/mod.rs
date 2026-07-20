use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::{QueryBuilder, Sqlite, Transaction};
use synd_feed::{
    entry::EntryId,
    types::{Annotated, Category, FeedMeta, FeedUrl, Requirement},
};
use synd_registry::{
    RegistryDbResult,
    db::TimelineDb,
    entry::EntryAttrs,
    query::{
        TimelineChange, TimelineChangesPage, TimelineChangesQuery, TimelineEntriesPage,
        TimelineEntriesQuery, TimelineEntry, TimelineEntryCursor,
    },
    subscription::{SubscriberId, SubscriptionKey},
    timeline::TimelineCatchup,
};

use super::{
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    pagination::PageLimit,
};

const TIMELINE_ENTRY_SELECT: &str = r#"
SELECT
    te.order_time,
    te.entry_id,
    e.attrs_json,
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
    let candidates = sqlx::query_scalar::<_, i64>(
        r#"
            SELECT COUNT(*)
            FROM entry AS e
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            WHERE f.url = ?
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_one(&mut **tx)
    .await?;
    if candidates == 0 {
        return Ok(TimelineCatchup::new(
            subscriber_id.clone(),
            feed_url.clone(),
            0,
        ));
    }

    // Every row gets its own seq: change pagination pages by seq alone, so
    // rows sharing a seq would be lost at page boundaries. Candidates left
    // untouched leave gaps, which is fine because seq only needs to be
    // unique and increasing.
    let base_seq = alloc_seq_range(tx, subscriber_id, candidates).await?;
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
            FROM entry AS e
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            WHERE f.url = ?
            ON CONFLICT (subscriber_id, entry_id) DO UPDATE SET
                seq = excluded.seq,
                deleted = 0
            WHERE timeline_entry.deleted != 0
            "#,
    )
    .bind(subscriber_id.as_str())
    .bind(base_seq)
    .bind(feed_url.as_str())
    .execute(&mut **tx)
    .await?;

    Ok(TimelineCatchup::new(
        subscriber_id.clone(),
        feed_url.clone(),
        result.rows_affected(),
    ))
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
    let Some(last_seq) = load_last_seq(tx, &query.subscriber_id).await? else {
        return Ok(TimelineEntriesPage {
            nodes: Vec::new(),
            has_next_page: false,
            end_cursor: None,
            seq: 0,
        });
    };

    let page_limit = PageLimit::new(query.first);
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
    sql.push_bind(page_limit.sql_limit());

    let rows = sql
        .build_query_as::<TimelineEntryRow>()
        .fetch_all(&mut **tx)
        .await?;
    let mut nodes = rows
        .into_iter()
        .map(TimelineEntryRow::into_node)
        .collect::<SqliteResult<Vec<_>>>()?;
    let has_next_page = page_limit.truncate_overfetch(&mut nodes);
    let end_cursor = nodes.last().map(|node| node.cursor.clone());

    Ok(TimelineEntriesPage {
        nodes,
        has_next_page,
        end_cursor,
        seq: last_seq,
    })
}

async fn list_changes(
    tx: &mut Transaction<'_, Sqlite>,
    query: TimelineChangesQuery,
) -> SqliteResult<TimelineChangesPage> {
    let Some(last_seq) = load_last_seq(tx, &query.subscriber_id).await? else {
        return Ok(TimelineChangesPage {
            changes: Vec::new(),
            seq: 0,
            has_more: false,
        });
    };

    let page_limit = PageLimit::new(query.limit);
    // Tombstoned entries have lost their subscription, so subscription and
    // snapshot columns are joined optionally and their absence also means
    // removal
    let mut rows = sqlx::query_as::<_, TimelineChangeRow>(
        r#"
            SELECT
                te.order_time,
                te.entry_id,
                te.seq,
                (te.deleted != 0 OR s.subscriber_id IS NULL) AS removed,
                e.attrs_json,
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
    .bind(page_limit.sql_limit())
    .fetch_all(&mut **tx)
    .await?;

    let has_more = page_limit.truncate_overfetch(&mut rows);
    let seq = if has_more {
        rows.last().map_or(query.since, |row| row.seq)
    } else {
        last_seq
    };
    let changes = rows
        .into_iter()
        .map(TimelineChangeRow::into_change)
        .collect::<SqliteResult<Vec<_>>>()?;

    Ok(TimelineChangesPage {
        changes,
        seq,
        has_more,
    })
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
            INNER JOIN entry AS e
                ON e.feed_pk = f.pk
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

    Ok(rows
        .into_iter()
        .map(TimelineEntryTargetRow::into_target)
        .collect())
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

impl TimelineEntryTargetRow {
    fn into_target(self) -> TimelineEntryTarget {
        TimelineEntryTarget {
            subscriber_id: SubscriberId::new(self.subscriber_id),
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
    attrs_json: String,
    meta_json: Option<String>,
    requirement: Option<String>,
    category: Option<String>,
}

impl TimelineChangeRow {
    fn into_change(self) -> SqliteResult<TimelineChange> {
        if self.removed {
            let entry_id = EntryId::parse(self.entry_id).decode()?;
            return Ok(TimelineChange::Remove { entry_id });
        }

        let meta_json = self.meta_json.ok_or_else(|| {
            SqliteError::decode_message("timeline change row without feed snapshot")
        })?;
        let row = TimelineEntryRow {
            order_time: self.order_time,
            entry_id: self.entry_id,
            attrs_json: self.attrs_json,
            meta_json,
            requirement: self.requirement,
            category: self.category,
        };
        Ok(TimelineChange::Upsert(Box::new(row.into_node()?)))
    }
}

#[derive(sqlx::FromRow)]
struct TimelineEntryRow {
    order_time: DateTime<Utc>,
    entry_id: String,
    attrs_json: String,
    meta_json: String,
    requirement: Option<String>,
    category: Option<String>,
}

impl TimelineEntryRow {
    fn into_node(self) -> SqliteResult<TimelineEntry> {
        let entry_id = EntryId::parse(self.entry_id).decode()?;
        let attrs = serde_json::from_str::<EntryAttrs>(&self.attrs_json)?;
        let feed_meta = serde_json::from_str::<FeedMeta>(&self.meta_json)?;
        let requirement = self
            .requirement
            .as_deref()
            .map(Requirement::from_str)
            .transpose()
            .decode()?;
        let category = self.category.map(Category::new).transpose().decode()?;
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
