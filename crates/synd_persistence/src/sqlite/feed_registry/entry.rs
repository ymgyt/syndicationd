use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::{EntryId, FeedUrl};
use synd_registry::{
    EntryProjectionTx, RegistryDbResult,
    crawl::{job::CrawlJobId, result::CrawlResultRef},
    entry::{
        Entry, EntryChange, EntryChanges, EntryLifecycle, EntryOrderKey, EntrySet, EntrySourceRef,
    },
    feed::FeedSource,
};

use super::{
    codec::{decode_entry_attrs_json, encode_entry_attrs_json},
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    feed,
};

async fn load_entries(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
    entry_ids: &[EntryId],
) -> SqliteResult<EntrySet> {
    if entry_ids.is_empty() {
        return Ok(EntrySet::empty(feed_url.clone()));
    }

    let entry_id_values = entry_ids.iter().map(EntryId::as_str).collect::<Vec<_>>();
    let entry_ids_json = serde_json::to_string(&entry_id_values)?;
    let rows = sqlx::query_as::<_, EntryRow>(
        r#"
            WITH requested(entry_id) AS (
                SELECT CAST(value AS TEXT)
                FROM json_each(?)
            )
            SELECT
                e.entry_id,
                e.current_content_json,
                e.current_order_time,
                e.current_source_result_pk,
                e.first_seen_at,
                e.last_seen_at,
                e.updated_at,
                cr.job_id AS source_job_id
            FROM requested AS r
            INNER JOIN entry AS e
                ON e.entry_id = r.entry_id
            INNER JOIN feed AS f
                ON f.pk = e.feed_pk
            INNER JOIN feed_endpoint AS fe
                ON fe.pk = f.feed_endpoint_pk
            INNER JOIN crawl_result AS cr
                ON cr.pk = e.current_source_result_pk
            WHERE fe.url = ?
            ORDER BY e.entry_id
            "#,
    )
    .bind(entry_ids_json)
    .bind(feed_url.as_str())
    .fetch_all(&mut **tx)
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| row.into_entry(feed_url))
        .collect::<SqliteResult<Vec<_>>>()?;
    Ok(EntrySet::new(feed_url.clone(), entries))
}

async fn apply_entry_changes(
    tx: &mut Transaction<'_, Sqlite>,
    changes: EntryChanges,
) -> SqliteResult<()> {
    for change in changes.into_changes() {
        match change {
            EntryChange::Discovered(entry) => insert_entry(tx, &entry).await?,
            EntryChange::Changed(entry) | EntryChange::AlreadySeen(entry) => {
                update_entry(tx, &entry).await?;
            }
        }
    }
    Ok(())
}

async fn insert_entry(tx: &mut Transaction<'_, Sqlite>, entry: &Entry) -> SqliteResult<()> {
    let feed_pk = resolve_feed_pk(tx, &entry.feed_url).await?;
    let attrs_json = encode_entry_attrs_json(&entry.attrs)?;
    sqlx::query(
        r#"
            INSERT INTO entry (
                feed_pk,
                entry_id,
                current_content_json,
                current_order_time,
                current_source_result_pk,
                first_seen_at,
                last_seen_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
    )
    .bind(feed_pk)
    .bind(entry.id.as_str())
    .bind(attrs_json)
    .bind(entry.order_key.as_datetime())
    .bind(entry.source.result_ref.pk())
    .bind(entry.lifecycle.first_seen_at)
    .bind(entry.lifecycle.last_seen_at)
    .bind(entry.lifecycle.updated_at)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn update_entry(tx: &mut Transaction<'_, Sqlite>, entry: &Entry) -> SqliteResult<()> {
    let attrs_json = encode_entry_attrs_json(&entry.attrs)?;
    let result = sqlx::query(
        r#"
            UPDATE entry
            SET
                current_content_json = ?,
                current_order_time = ?,
                current_source_result_pk = ?,
                last_seen_at = ?,
                updated_at = ?
            WHERE entry_id = ?
              AND feed_pk = (
                SELECT f.pk
                FROM feed AS f
                INNER JOIN feed_endpoint AS fe
                    ON fe.pk = f.feed_endpoint_pk
                WHERE fe.url = ?
              )
            "#,
    )
    .bind(attrs_json)
    .bind(entry.order_key.as_datetime())
    .bind(entry.source.result_ref.pk())
    .bind(entry.lifecycle.last_seen_at)
    .bind(entry.lifecycle.updated_at)
    .bind(entry.id.as_str())
    .bind(entry.feed_url.as_str())
    .execute(&mut **tx)
    .await?;

    if result.rows_affected() != 1 {
        return Err(SqliteError::decode_message(format!(
            "entry update affected {} rows for {}",
            result.rows_affected(),
            entry.id
        )));
    }
    Ok(())
}

async fn resolve_feed_pk(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<i64> {
    let row = sqlx::query_as::<_, FeedPkRow>(
        r#"
            SELECT f.pk
            FROM feed AS f
            INNER JOIN feed_endpoint AS fe
                ON fe.pk = f.feed_endpoint_pk
            WHERE fe.url = ?
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Err(SqliteError::not_found(
            "feed for entry projection",
            feed_url.as_str(),
        ));
    };
    Ok(row.pk)
}

#[derive(sqlx::FromRow)]
struct EntryRow {
    entry_id: String,
    current_content_json: String,
    current_order_time: DateTime<Utc>,
    current_source_result_pk: i64,
    first_seen_at: DateTime<Utc>,
    last_seen_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    source_job_id: String,
}

impl EntryRow {
    fn into_entry(self, feed_url: &FeedUrl) -> SqliteResult<Entry> {
        Ok(Entry::builder()
            .id(EntryId::parse(self.entry_id).decode()?)
            .feed_url(feed_url.clone())
            .attrs(decode_entry_attrs_json(&self.current_content_json)?)
            .order_key(EntryOrderKey::from_datetime(self.current_order_time))
            .lifecycle(
                EntryLifecycle::builder()
                    .first_seen_at(self.first_seen_at)
                    .last_seen_at(self.last_seen_at)
                    .updated_at(self.updated_at)
                    .build(),
            )
            .source(
                EntrySourceRef::builder()
                    .crawl_job_id(CrawlJobId::new(self.source_job_id))
                    .result_ref(CrawlResultRef::new(self.current_source_result_pk))
                    .build(),
            )
            .build())
    }
}

#[derive(sqlx::FromRow)]
struct FeedPkRow {
    pk: i64,
}

impl EntryProjectionTx for super::SqliteRegistryTx<'_> {
    async fn load_entry_source(
        &mut self,
        job_id: &CrawlJobId,
    ) -> RegistryDbResult<Option<FeedSource>> {
        feed::load_source(&mut self.tx, job_id).await.db()
    }

    async fn load_entries(
        &mut self,
        feed_url: &FeedUrl,
        entry_ids: &[EntryId],
    ) -> RegistryDbResult<EntrySet> {
        load_entries(&mut self.tx, feed_url, entry_ids).await.db()
    }

    async fn apply_entry_changes(&mut self, changes: EntryChanges) -> RegistryDbResult<()> {
        apply_entry_changes(&mut self.tx, changes).await.db()
    }
}

#[cfg(test)]
mod tests;
