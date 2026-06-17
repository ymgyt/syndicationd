use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    FeedStore, RegistryDbResult,
    crawl::{blob::BlobRef, job::CrawlJobId, result::CrawlResultRef},
    feed::{FeedSource, UpsertFeedCommand, UpsertFeedOutcome},
};

use super::{
    error::{DecodeResultExt, IntoDbResult, SqliteResult},
    feed_endpoint,
};

pub(super) async fn load_source(
    tx: &mut Transaction<'_, Sqlite>,
    job_id: &CrawlJobId,
) -> SqliteResult<Option<FeedSource>> {
    let row = sqlx::query_as::<_, FeedSourceRow>(
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
    .fetch_optional(&mut **tx)
    .await?;

    row.map(FeedSourceRow::into_source).transpose()
}

async fn upsert_current(
    tx: &mut Transaction<'_, Sqlite>,
    command: UpsertFeedCommand,
) -> SqliteResult<UpsertFeedOutcome> {
    let source = command.source;
    let feed_endpoint_pk = feed_endpoint::resolve_pk(tx, &source.feed_url).await?;
    let meta_json = serde_json::to_string(&command.meta)?;
    let previous = load_current(tx, feed_endpoint_pk).await?;
    let outcome = match previous {
        None => {
            insert_current(tx, feed_endpoint_pk, &source, &meta_json).await?;
            UpsertFeedOutcome::Discovered
        }
        Some(previous) => {
            update_current(tx, feed_endpoint_pk, &source, &meta_json).await?;
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
    tx: &mut Transaction<'_, Sqlite>,
    feed_endpoint_pk: i64,
) -> SqliteResult<Option<StoredFeed>> {
    let row = sqlx::query_as::<_, StoredFeed>(
        r#"
            SELECT current_meta_json, current_body_blob_pk
            FROM feed
            WHERE feed_endpoint_pk = ?
            "#,
    )
    .bind(feed_endpoint_pk)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row)
}

async fn insert_current(
    tx: &mut Transaction<'_, Sqlite>,
    feed_endpoint_pk: i64,
    source: &FeedSource,
    meta_json: &str,
) -> SqliteResult<()> {
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
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn update_current(
    tx: &mut Transaction<'_, Sqlite>,
    feed_endpoint_pk: i64,
    source: &FeedSource,
    meta_json: &str,
) -> SqliteResult<()> {
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
    .execute(&mut **tx)
    .await?;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct FeedSourceRow {
    result_pk: i64,
    job_id: String,
    feed_url: String,
    finished_at: DateTime<Utc>,
    body_blob_pk: i64,
}

impl FeedSourceRow {
    fn into_source(self) -> SqliteResult<FeedSource> {
        Ok(FeedSource::builder()
            .feed_url(FeedUrl::parse(&self.feed_url).decode()?)
            .crawl_job_id(CrawlJobId::new(self.job_id))
            .result_ref(CrawlResultRef::new(self.result_pk))
            .body_blob(BlobRef::new(self.body_blob_pk))
            .seen_at(self.finished_at)
            .build())
    }
}

#[derive(sqlx::FromRow)]
struct StoredFeed {
    current_meta_json: String,
    current_body_blob_pk: i64,
}

impl FeedStore for super::SqliteRegistryTx<'_> {
    async fn upsert_feed(
        &mut self,
        command: UpsertFeedCommand,
    ) -> RegistryDbResult<UpsertFeedOutcome> {
        upsert_current(&mut self.tx, command).await.db()
    }
}

#[cfg(test)]
mod tests;
