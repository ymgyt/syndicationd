use chrono::{DateTime, Utc};
use sqlx::{Row, Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    FeedProjectionTx, RegistryDbError, RegistryDbResult,
    crawl::{blob::BlobRef, job::CrawlJobId, result::CrawlResultRef},
    feed::{FeedSource, UpsertFeedCommand, UpsertFeedOutcome},
};

use super::feed_endpoint::FeedEndpointTable;

pub(super) struct FeedTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> FeedTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn load_source(
        &mut self,
        job_id: &CrawlJobId,
    ) -> RegistryDbResult<Option<FeedSource>> {
        let row = sqlx::query(
            r#"
            SELECT
                cr.pk AS result_pk,
                cr.job_id,
                e.url AS feed_url,
                cr.finished_at,
                h.body_blob_pk
            FROM crawl_result AS cr
            INNER JOIN feed_endpoint AS e
                ON e.pk = cr.feed_endpoint_pk
            INNER JOIN crawl_http_response AS h
                ON h.result_pk = cr.pk
            LEFT JOIN crawl_feed_parse_error AS pe
                ON pe.result_pk = cr.pk
            WHERE cr.job_id = ?
              AND h.status_code BETWEEN 200 AND 299
              AND h.body_blob_pk IS NOT NULL
              AND pe.result_pk IS NULL
            "#,
        )
        .bind(job_id.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Ok(None);
        };
        let feed_url = row
            .try_get::<String, _>("feed_url")
            .map_err(RegistryDbError::internal)?;
        let feed_url = FeedUrl::parse(&feed_url).map_err(RegistryDbError::internal)?;
        Ok(Some(
            FeedSource::builder()
                .feed_url(feed_url)
                .crawl_job_id(CrawlJobId::new(
                    row.try_get::<String, _>("job_id")
                        .map_err(RegistryDbError::internal)?,
                ))
                .result_ref(CrawlResultRef::new(
                    row.try_get::<i64, _>("result_pk")
                        .map_err(RegistryDbError::internal)?,
                ))
                .body_blob(BlobRef::new(
                    row.try_get::<i64, _>("body_blob_pk")
                        .map_err(RegistryDbError::internal)?,
                ))
                .seen_at(
                    row.try_get::<DateTime<Utc>, _>("finished_at")
                        .map_err(RegistryDbError::internal)?,
                )
                .build(),
        ))
    }

    pub(super) async fn upsert_current(
        &mut self,
        command: UpsertFeedCommand,
    ) -> RegistryDbResult<UpsertFeedOutcome> {
        let source = command.source;
        let feed_endpoint_pk = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_pk(&source.feed_url).await?
        };
        let meta_json = serde_json::to_string(&command.meta).map_err(RegistryDbError::internal)?;
        let previous = self.load_current(feed_endpoint_pk).await?;
        let outcome = match previous {
            None => {
                self.insert_current(feed_endpoint_pk, &source, &meta_json)
                    .await?;
                UpsertFeedOutcome::Discovered
            }
            Some(previous) => {
                self.update_current(feed_endpoint_pk, &source, &meta_json)
                    .await?;
                if previous.current_meta_json != meta_json
                    || previous.current_body_blob_pk != source.body_blob.pk()
                {
                    UpsertFeedOutcome::Changed
                } else {
                    UpsertFeedOutcome::Unchanged
                }
            }
        };
        Ok(outcome)
    }

    async fn load_current(
        &mut self,
        feed_endpoint_pk: i64,
    ) -> RegistryDbResult<Option<StoredFeed>> {
        let row = sqlx::query(
            r#"
            SELECT current_meta_json, current_body_blob_pk
            FROM feed
            WHERE feed_endpoint_pk = ?
            "#,
        )
        .bind(feed_endpoint_pk)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(StoredFeed {
            current_meta_json: row
                .try_get("current_meta_json")
                .map_err(RegistryDbError::internal)?,
            current_body_blob_pk: row
                .try_get("current_body_blob_pk")
                .map_err(RegistryDbError::internal)?,
        }))
    }

    async fn insert_current(
        &mut self,
        feed_endpoint_pk: i64,
        source: &FeedSource,
        meta_json: &str,
    ) -> RegistryDbResult<()> {
        sqlx::query(
            r#"
            INSERT INTO feed (
                feed_endpoint_pk,
                current_meta_json,
                current_body_blob_pk,
                current_source_result_pk,
                first_seen_at,
                last_seen_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(feed_endpoint_pk)
        .bind(meta_json)
        .bind(source.body_blob.pk())
        .bind(source.result_ref.pk())
        .bind(source.seen_at)
        .bind(source.seen_at)
        .bind(source.seen_at)
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;
        Ok(())
    }

    async fn update_current(
        &mut self,
        feed_endpoint_pk: i64,
        source: &FeedSource,
        meta_json: &str,
    ) -> RegistryDbResult<()> {
        sqlx::query(
            r#"
            UPDATE feed
            SET
                current_meta_json = ?,
                current_body_blob_pk = ?,
                current_source_result_pk = ?,
                last_seen_at = ?,
                updated_at = ?
            WHERE feed_endpoint_pk = ?
            "#,
        )
        .bind(meta_json)
        .bind(source.body_blob.pk())
        .bind(source.result_ref.pk())
        .bind(source.seen_at)
        .bind(source.seen_at)
        .bind(feed_endpoint_pk)
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;
        Ok(())
    }
}

impl FeedProjectionTx for super::SqliteRegistryTx<'_> {
    async fn load_feed_source(
        &mut self,
        job_id: &CrawlJobId,
    ) -> RegistryDbResult<Option<FeedSource>> {
        FeedTable::new(&mut self.tx).load_source(job_id).await
    }

    async fn upsert_feed(
        &mut self,
        command: UpsertFeedCommand,
    ) -> RegistryDbResult<UpsertFeedOutcome> {
        FeedTable::new(&mut self.tx).upsert_current(command).await
    }
}

struct StoredFeed {
    current_meta_json: String,
    current_body_blob_pk: i64,
}
