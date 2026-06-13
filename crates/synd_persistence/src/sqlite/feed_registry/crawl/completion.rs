use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::{
    feed::service::{FeedConditionalFetch, FeedHttpStatus},
    types::FeedUrl,
};
use synd_registry::{
    CrawlCompletionTx, RegistryDbResult,
    crawl::result::{
        CrawlFeedParseErrorDetail, CrawlFetchErrorDetail, CrawlHealth, CrawlHttpBodyDetail,
        CrawlHttpResponseDetail, CrawlResultDetail, CrawlResultRef, CrawlState, CrawlStateError,
        CrawlStateTimestamps, FailureStreak, LastCrawlResult, RecordCrawlResultCommand,
        UpsertCrawlStateCommand,
    },
};

use super::super::{
    SqliteRegistryTx, codec,
    error::{IntoDbResult, SqliteError, SqliteResult},
    feed_endpoint,
};

async fn load_state(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<Option<CrawlState>> {
    let row = sqlx::query_as::<_, CrawlStateRow>(
        r#"
            SELECT
                cs.last_started_at,
                cs.last_finished_at,
                cs.last_http_status,
                cs.last_error_kind,
                cs.failure_streak,
                cs.last_retry_after,
                cs.etag,
                cs.last_modified,
                cs.created_at,
                cs.updated_at
            FROM crawl_state AS cs
            INNER JOIN feed_endpoint AS e
                ON e.pk = cs.feed_endpoint_pk
            WHERE e.url = ?
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    row.map(|row| row.into_state(feed_url)).transpose()
}

async fn record(
    tx: &mut Transaction<'_, Sqlite>,
    command: RecordCrawlResultCommand,
) -> SqliteResult<CrawlResultRef> {
    let feed_endpoint_pk = feed_endpoint::resolve_pk(tx, &command.record.feed_url).await?;

    let row = sqlx::query_as::<_, PkRow>(
        r#"
            INSERT INTO crawl_result (
                job_id,
                feed_endpoint_pk,
                started_at,
                finished_at,
                created_at
            )
            VALUES (?, ?, ?, ?, ?)
            RETURNING pk
            "#,
    )
    .bind(command.record.job_id.as_str())
    .bind(feed_endpoint_pk)
    .bind(command.record.started_at)
    .bind(command.record.finished_at)
    .bind(command.record.finished_at)
    .fetch_one(&mut **tx)
    .await?;

    let result_ref = CrawlResultRef::new(row.pk);
    insert_detail(tx, result_ref, command.detail).await?;
    Ok(result_ref)
}

async fn upsert_state(
    tx: &mut Transaction<'_, Sqlite>,
    command: UpsertCrawlStateCommand,
) -> SqliteResult<()> {
    let feed_endpoint_pk = feed_endpoint::resolve_pk(tx, &command.feed_url).await?;
    let last_http_status = command
        .last
        .http_status
        .map(|status| i64::from(status.as_u16()));
    let last_error_kind = command
        .last
        .error
        .map(|error| codec::encode_crawl_state_error_kind(error.kind));
    let failure_streak = encode_u64(command.health.failure_streak.value(), "failure streak")?;

    sqlx::query(
        r#"
            INSERT INTO crawl_state (
                feed_endpoint_pk,
                last_result_pk,
                last_started_at,
                last_finished_at,
                last_http_status,
                last_error_kind,
                failure_streak,
                last_retry_after,
                etag,
                last_modified,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(feed_endpoint_pk) DO UPDATE SET
                last_result_pk = excluded.last_result_pk,
                last_started_at = excluded.last_started_at,
                last_finished_at = excluded.last_finished_at,
                last_http_status = excluded.last_http_status,
                last_error_kind = excluded.last_error_kind,
                failure_streak = excluded.failure_streak,
                last_retry_after = excluded.last_retry_after,
                etag = excluded.etag,
                last_modified = excluded.last_modified,
                updated_at = excluded.updated_at
            "#,
    )
    .bind(feed_endpoint_pk)
    .bind(command.last_result.pk())
    .bind(command.last.started_at)
    .bind(command.last.finished_at)
    .bind(last_http_status)
    .bind(last_error_kind)
    .bind(failure_streak)
    .bind(command.last.retry_after)
    .bind(command.conditional.etag)
    .bind(command.conditional.last_modified)
    .bind(command.updated_at)
    .bind(command.updated_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_detail(
    tx: &mut Transaction<'_, Sqlite>,
    result_ref: CrawlResultRef,
    detail: CrawlResultDetail,
) -> SqliteResult<()> {
    match detail {
        CrawlResultDetail::Fetched { http, body }
        | CrawlResultDetail::UnexpectedStatus { http, body } => {
            insert_http_response(tx, result_ref, http, Some(body)).await
        }
        CrawlResultDetail::NotModified { http } => {
            insert_http_response(tx, result_ref, http, None).await
        }
        CrawlResultDetail::BodyReadFailed { http, error } => {
            insert_http_response(tx, result_ref, http, None).await?;
            insert_fetch_error(tx, result_ref, error).await
        }
        CrawlResultDetail::FetchFailed { error } => insert_fetch_error(tx, result_ref, error).await,
        CrawlResultDetail::ParseFailed { http, body, error } => {
            insert_http_response(tx, result_ref, http, Some(body)).await?;
            insert_feed_parse_error(tx, result_ref, error).await
        }
    }
}

async fn insert_http_response(
    tx: &mut Transaction<'_, Sqlite>,
    result_ref: CrawlResultRef,
    http: CrawlHttpResponseDetail,
    body: Option<CrawlHttpBodyDetail>,
) -> SqliteResult<()> {
    let content_length = http
        .content_length
        .map(|value| encode_u64(value, "HTTP content length"))
        .transpose()?;

    sqlx::query(
        r#"
            INSERT INTO crawl_http_response (
                result_pk,
                status_code,
                response_url,
                headers_blob_pk,
                body_blob_pk,
                content_type,
                content_length,
                etag,
                last_modified,
                retry_after_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
    )
    .bind(result_ref.pk())
    .bind(i64::from(http.status.as_u16()))
    .bind(http.response_url.as_str())
    .bind(http.headers_blob.pk())
    .bind(body.map(|body| body.body_blob.pk()))
    .bind(http.content_type)
    .bind(content_length)
    .bind(http.etag)
    .bind(http.last_modified)
    .bind(http.retry_after_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_fetch_error(
    tx: &mut Transaction<'_, Sqlite>,
    result_ref: CrawlResultRef,
    error: CrawlFetchErrorDetail,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            INSERT INTO crawl_fetch_error (
                result_pk,
                error_kind,
                error_message
            )
            VALUES (?, ?, ?)
            "#,
    )
    .bind(result_ref.pk())
    .bind(error.kind.as_str())
    .bind(error.message)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn insert_feed_parse_error(
    tx: &mut Transaction<'_, Sqlite>,
    result_ref: CrawlResultRef,
    error: CrawlFeedParseErrorDetail,
) -> SqliteResult<()> {
    sqlx::query(
        r#"
            INSERT INTO crawl_feed_parse_error (
                result_pk,
                error_kind,
                error_message
            )
            VALUES (?, ?, ?)
            "#,
    )
    .bind(result_ref.pk())
    .bind(error.kind.as_str())
    .bind(error.message)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

#[derive(sqlx::FromRow)]
struct CrawlStateRow {
    last_started_at: DateTime<Utc>,
    last_finished_at: DateTime<Utc>,
    last_http_status: Option<i64>,
    last_error_kind: Option<String>,
    failure_streak: i64,
    last_retry_after: Option<DateTime<Utc>>,
    etag: Option<String>,
    last_modified: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl CrawlStateRow {
    fn into_state(self, feed_url: &FeedUrl) -> SqliteResult<CrawlState> {
        let last_error_kind = self
            .last_error_kind
            .map(|kind| {
                codec::decode_crawl_state_error_kind(&kind).map(|kind| CrawlStateError { kind })
            })
            .transpose()?;
        let http_status = self.last_http_status.map(decode_http_status).transpose()?;
        let failure_streak = decode_failure_streak(self.failure_streak)?;

        Ok(CrawlState {
            feed_url: feed_url.clone(),
            last: LastCrawlResult {
                started_at: self.last_started_at,
                finished_at: self.last_finished_at,
                http_status,
                error: last_error_kind,
                retry_after: self.last_retry_after,
            },
            health: CrawlHealth { failure_streak },
            conditional: FeedConditionalFetch {
                etag: self.etag,
                last_modified: self.last_modified,
            },
            timestamps: CrawlStateTimestamps::new(self.created_at, self.updated_at),
        })
    }
}

#[derive(sqlx::FromRow)]
struct PkRow {
    pk: i64,
}

fn decode_http_status(value: i64) -> SqliteResult<FeedHttpStatus> {
    let status = u16::try_from(value)
        .map_err(|_| SqliteError::decode_message(format!("invalid HTTP status: {value}")))?;
    Ok(FeedHttpStatus::new(status))
}

fn decode_failure_streak(value: i64) -> SqliteResult<FailureStreak> {
    let value =
        u64::try_from(value).map_err(|_| SqliteError::decode_message("negative failure streak"))?;
    Ok(FailureStreak::new(value))
}

fn encode_u64(value: u64, field: &'static str) -> SqliteResult<i64> {
    i64::try_from(value)
        .map_err(|_| SqliteError::decode_message(format!("{field} exceeds SQLite INTEGER range")))
}

impl CrawlCompletionTx for SqliteRegistryTx<'_> {
    async fn load_crawl_state(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlState>> {
        load_state(&mut self.tx, feed_url).await.db()
    }

    async fn record_crawl_result(
        &mut self,
        command: RecordCrawlResultCommand,
    ) -> RegistryDbResult<CrawlResultRef> {
        record(&mut self.tx, command).await.db()
    }

    async fn upsert_crawl_state(
        &mut self,
        command: UpsertCrawlStateCommand,
    ) -> RegistryDbResult<()> {
        upsert_state(&mut self.tx, command).await.db()
    }
}

#[cfg(test)]
mod tests;
