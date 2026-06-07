use sqlx::{Row, Sqlite, Transaction};
use synd_feed::{
    feed::service::{FeedConditionalFetch, FeedHttpStatus},
    types::FeedUrl,
};
use synd_registry::{
    CrawlCompletionTx, RegistryDbError, RegistryDbResult,
    crawl::result::{
        CrawlFeedParseErrorDetail, CrawlFetchErrorDetail, CrawlHealth, CrawlHttpBodyDetail,
        CrawlHttpResponseDetail, CrawlResultDetail, CrawlResultRef, CrawlState, CrawlStateError,
        CrawlStateTimestamps, FailureStreak, LastCrawlResult, RecordCrawlResultCommand,
        UpsertCrawlStateCommand,
    },
};

use super::{codec, feed_endpoint::FeedEndpointTable};

pub(super) struct CrawlResultTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> CrawlResultTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn load_state(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlState>> {
        let row = sqlx::query(
            r#"
            SELECT
                cs.last_result_pk,
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
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Ok(None);
        };

        let last_error_kind = row
            .try_get::<Option<String>, _>("last_error_kind")
            .map_err(RegistryDbError::internal)?
            .map(|kind| {
                codec::decode_crawl_state_error_kind(&kind).map(|kind| CrawlStateError { kind })
            })
            .transpose()?;
        let http_status = row
            .try_get::<Option<i64>, _>("last_http_status")
            .map_err(RegistryDbError::internal)?
            .map(decode_http_status)
            .transpose()?;
        let failure_streak = row
            .try_get::<i64, _>("failure_streak")
            .map_err(RegistryDbError::internal)
            .and_then(decode_failure_streak)?;

        Ok(Some(CrawlState {
            feed_url: feed_url.clone(),
            last: LastCrawlResult {
                started_at: row
                    .try_get("last_started_at")
                    .map_err(RegistryDbError::internal)?,
                finished_at: row
                    .try_get("last_finished_at")
                    .map_err(RegistryDbError::internal)?,
                http_status,
                error: last_error_kind,
                retry_after: row
                    .try_get("last_retry_after")
                    .map_err(RegistryDbError::internal)?,
            },
            health: CrawlHealth { failure_streak },
            conditional: FeedConditionalFetch {
                etag: row.try_get("etag").map_err(RegistryDbError::internal)?,
                last_modified: row
                    .try_get("last_modified")
                    .map_err(RegistryDbError::internal)?,
            },
            timestamps: CrawlStateTimestamps::new(
                row.try_get("created_at")
                    .map_err(RegistryDbError::internal)?,
                row.try_get("updated_at")
                    .map_err(RegistryDbError::internal)?,
            ),
        }))
    }

    pub(super) async fn record(
        &mut self,
        command: RecordCrawlResultCommand,
    ) -> RegistryDbResult<CrawlResultRef> {
        let feed_endpoint_pk = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_pk(&command.record.feed_url).await?
        };

        let row = sqlx::query(
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
        .fetch_one(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let result_ref = CrawlResultRef::new(
            row.try_get::<i64, _>("pk")
                .map_err(RegistryDbError::internal)?,
        );
        self.insert_detail(result_ref, command.detail).await?;
        Ok(result_ref)
    }

    pub(super) async fn upsert_state(
        &mut self,
        command: UpsertCrawlStateCommand,
    ) -> RegistryDbResult<()> {
        let feed_endpoint_pk = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_pk(&command.feed_url).await?
        };
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn insert_detail(
        &mut self,
        result_ref: CrawlResultRef,
        detail: CrawlResultDetail,
    ) -> RegistryDbResult<()> {
        match detail {
            CrawlResultDetail::Fetched { http, body }
            | CrawlResultDetail::UnexpectedStatus { http, body } => {
                self.insert_http_response(result_ref, http, Some(body))
                    .await
            }
            CrawlResultDetail::NotModified { http } => {
                self.insert_http_response(result_ref, http, None).await
            }
            CrawlResultDetail::BodyReadFailed { http, error } => {
                self.insert_http_response(result_ref, http, None).await?;
                self.insert_fetch_error(result_ref, error).await
            }
            CrawlResultDetail::FetchFailed { error } => {
                self.insert_fetch_error(result_ref, error).await
            }
            CrawlResultDetail::ParseFailed { http, body, error } => {
                self.insert_http_response(result_ref, http, Some(body))
                    .await?;
                self.insert_feed_parse_error(result_ref, error).await
            }
        }
    }

    async fn insert_http_response(
        &mut self,
        result_ref: CrawlResultRef,
        http: CrawlHttpResponseDetail,
        body: Option<CrawlHttpBodyDetail>,
    ) -> RegistryDbResult<()> {
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn insert_fetch_error(
        &mut self,
        result_ref: CrawlResultRef,
        error: CrawlFetchErrorDetail,
    ) -> RegistryDbResult<()> {
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }

    async fn insert_feed_parse_error(
        &mut self,
        result_ref: CrawlResultRef,
        error: CrawlFeedParseErrorDetail,
    ) -> RegistryDbResult<()> {
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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }
}

impl CrawlCompletionTx for super::SqliteRegistryTx<'_> {
    async fn load_crawl_state(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlState>> {
        CrawlResultTable::new(&mut self.tx)
            .load_state(feed_url)
            .await
    }

    async fn record_crawl_result(
        &mut self,
        command: RecordCrawlResultCommand,
    ) -> RegistryDbResult<CrawlResultRef> {
        CrawlResultTable::new(&mut self.tx).record(command).await
    }

    async fn upsert_crawl_state(
        &mut self,
        command: UpsertCrawlStateCommand,
    ) -> RegistryDbResult<()> {
        CrawlResultTable::new(&mut self.tx)
            .upsert_state(command)
            .await
    }
}

fn decode_http_status(value: i64) -> RegistryDbResult<FeedHttpStatus> {
    let status = u16::try_from(value)
        .map_err(|_| RegistryDbError::internal(anyhow::anyhow!("invalid HTTP status: {value}")))?;
    Ok(FeedHttpStatus::new(status))
}

fn decode_failure_streak(value: i64) -> RegistryDbResult<FailureStreak> {
    let value = u64::try_from(value)
        .map_err(|_| RegistryDbError::internal(anyhow::anyhow!("negative failure streak")))?;
    Ok(FailureStreak::new(value))
}

fn encode_u64(value: u64, field: &'static str) -> RegistryDbResult<i64> {
    i64::try_from(value).map_err(|_| {
        RegistryDbError::internal(anyhow::anyhow!("{field} exceeds SQLite INTEGER range"))
    })
}
