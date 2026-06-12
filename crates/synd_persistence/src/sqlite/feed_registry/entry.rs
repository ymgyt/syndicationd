use chrono::{DateTime, Utc};
use sqlx::{Row, Sqlite, Transaction};
use synd_feed::types::{EntryId, FeedUrl};
use synd_registry::{
    EntryProjectionTx, RegistryDbError, RegistryDbResult,
    crawl::{job::CrawlJobId, result::CrawlResultRef},
    entry::{
        Entry, EntryChange, EntryChanges, EntryLifecycle, EntryOrderKey, EntrySet, EntrySourceRef,
    },
    feed::FeedSource,
};

use super::{
    codec::{decode_entry_attrs_json, encode_entry_attrs_json},
    feed::FeedTable,
};

pub(super) struct EntryTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> EntryTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn load_entries(
        &mut self,
        feed_url: &FeedUrl,
        entry_ids: &[EntryId],
    ) -> RegistryDbResult<EntrySet> {
        if entry_ids.is_empty() {
            return Ok(EntrySet::empty(feed_url.clone()));
        }

        let entry_id_values = entry_ids.iter().map(EntryId::as_str).collect::<Vec<_>>();
        let entry_ids_json =
            serde_json::to_string(&entry_id_values).map_err(RegistryDbError::internal)?;
        let rows = sqlx::query(
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
        .fetch_all(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            let entry_id = row
                .try_get::<String, _>("entry_id")
                .map_err(RegistryDbError::internal)?;
            let attrs_json = row
                .try_get::<String, _>("current_content_json")
                .map_err(RegistryDbError::internal)?;
            entries.push(
                Entry::builder()
                    .id(EntryId::parse(entry_id).map_err(RegistryDbError::internal)?)
                    .feed_url(feed_url.clone())
                    .attrs(decode_entry_attrs_json(&attrs_json)?)
                    .order_key(EntryOrderKey::from_datetime(
                        row.try_get::<DateTime<Utc>, _>("current_order_time")
                            .map_err(RegistryDbError::internal)?,
                    ))
                    .lifecycle(
                        EntryLifecycle::builder()
                            .first_seen_at(
                                row.try_get::<DateTime<Utc>, _>("first_seen_at")
                                    .map_err(RegistryDbError::internal)?,
                            )
                            .last_seen_at(
                                row.try_get::<DateTime<Utc>, _>("last_seen_at")
                                    .map_err(RegistryDbError::internal)?,
                            )
                            .updated_at(
                                row.try_get::<DateTime<Utc>, _>("updated_at")
                                    .map_err(RegistryDbError::internal)?,
                            )
                            .build(),
                    )
                    .source(
                        EntrySourceRef::builder()
                            .crawl_job_id(CrawlJobId::new(
                                row.try_get::<String, _>("source_job_id")
                                    .map_err(RegistryDbError::internal)?,
                            ))
                            .result_ref(CrawlResultRef::new(
                                row.try_get::<i64, _>("current_source_result_pk")
                                    .map_err(RegistryDbError::internal)?,
                            ))
                            .build(),
                    )
                    .build(),
            );
        }

        Ok(EntrySet::new(feed_url.clone(), entries))
    }

    pub(super) async fn apply_entry_changes(
        &mut self,
        changes: EntryChanges,
    ) -> RegistryDbResult<()> {
        for change in changes.into_changes() {
            match change {
                EntryChange::Discovered(entry) => self.insert_entry(&entry).await?,
                EntryChange::Changed(entry) | EntryChange::AlreadySeen(entry) => {
                    self.update_entry(&entry).await?;
                }
            }
        }
        Ok(())
    }

    async fn insert_entry(&mut self, entry: &Entry) -> RegistryDbResult<()> {
        let feed_pk = self.resolve_feed_pk(&entry.feed_url).await?;
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;
        Ok(())
    }

    async fn update_entry(&mut self, entry: &Entry) -> RegistryDbResult<()> {
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        if result.rows_affected() != 1 {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "entry update affected {} rows for {}",
                result.rows_affected(),
                entry.id
            )));
        }
        Ok(())
    }

    async fn resolve_feed_pk(&mut self, feed_url: &FeedUrl) -> RegistryDbResult<i64> {
        let row = sqlx::query(
            r#"
            SELECT f.pk
            FROM feed AS f
            INNER JOIN feed_endpoint AS fe
                ON fe.pk = f.feed_endpoint_pk
            WHERE fe.url = ?
            "#,
        )
        .bind(feed_url.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Err(RegistryDbError::internal(anyhow::anyhow!(
                "feed not found for entry projection: {feed_url}"
            )));
        };
        row.try_get::<i64, _>("pk")
            .map_err(RegistryDbError::internal)
    }
}

impl EntryProjectionTx for super::SqliteRegistryTx<'_> {
    async fn load_entry_source(
        &mut self,
        job_id: &CrawlJobId,
    ) -> RegistryDbResult<Option<FeedSource>> {
        FeedTable::new(&mut self.tx).load_source(job_id).await
    }

    async fn load_entries(
        &mut self,
        feed_url: &FeedUrl,
        entry_ids: &[EntryId],
    ) -> RegistryDbResult<EntrySet> {
        EntryTable::new(&mut self.tx)
            .load_entries(feed_url, entry_ids)
            .await
    }

    async fn apply_entry_changes(&mut self, changes: EntryChanges) -> RegistryDbResult<()> {
        EntryTable::new(&mut self.tx)
            .apply_entry_changes(changes)
            .await
    }
}
