use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::{
    feed::service::{FeedConditionalFetch, FeedHttpStatus},
    types::FeedUrl,
};
use synd_registry::{
    CrawlStateStore, RegistryDbResult,
    crawl::state::{
        CrawlHealth, CrawlState, CrawlStateError, FailureStreak, LastCrawlResult,
        UpsertCrawlStateCommand,
    },
};

use super::super::{
    SqliteRegistryTx, codec,
    error::{IntoDbResult, SqliteError, SqliteResult},
    feed,
};

async fn load(
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
                cs.retry_after,
                cs.etag,
                cs.last_modified
            FROM crawl_state AS cs
            INNER JOIN feed AS f
                ON f.pk = cs.feed_pk
            WHERE f.url = ?
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    row.map(|row| row.into_state(feed_url)).transpose()
}

async fn upsert(
    tx: &mut Transaction<'_, Sqlite>,
    command: UpsertCrawlStateCommand,
) -> SqliteResult<()> {
    let feed_pk = feed::resolve_pk(tx, &command.feed_url).await?;
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
                feed_pk,
                last_started_at,
                last_finished_at,
                last_http_status,
                last_error_kind,
                failure_streak,
                retry_after,
                etag,
                last_modified
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(feed_pk) DO UPDATE SET
                last_started_at = excluded.last_started_at,
                last_finished_at = excluded.last_finished_at,
                last_http_status = excluded.last_http_status,
                last_error_kind = excluded.last_error_kind,
                failure_streak = excluded.failure_streak,
                retry_after = excluded.retry_after,
                etag = excluded.etag,
                last_modified = excluded.last_modified
            "#,
    )
    .bind(feed_pk)
    .bind(command.last.started_at)
    .bind(command.last.finished_at)
    .bind(last_http_status)
    .bind(last_error_kind)
    .bind(failure_streak)
    .bind(command.last.retry_after)
    .bind(command.conditional.etag.as_deref())
    .bind(command.conditional.last_modified.as_deref())
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// `crawl_state` columns shared by the state load and the scheduler's due
/// input queries.
#[derive(sqlx::FromRow)]
pub(in crate::sqlite::feed_registry) struct CrawlStateRow {
    pub(in crate::sqlite::feed_registry) last_started_at: DateTime<Utc>,
    pub(in crate::sqlite::feed_registry) last_finished_at: DateTime<Utc>,
    pub(in crate::sqlite::feed_registry) last_http_status: Option<i64>,
    pub(in crate::sqlite::feed_registry) last_error_kind: Option<String>,
    pub(in crate::sqlite::feed_registry) failure_streak: i64,
    pub(in crate::sqlite::feed_registry) retry_after: Option<DateTime<Utc>>,
    pub(in crate::sqlite::feed_registry) etag: Option<String>,
    pub(in crate::sqlite::feed_registry) last_modified: Option<String>,
}

impl CrawlStateRow {
    pub(in crate::sqlite::feed_registry) fn into_state(
        self,
        feed_url: &FeedUrl,
    ) -> SqliteResult<CrawlState> {
        let http_status = self
            .last_http_status
            .map(|status| {
                u16::try_from(status).map(FeedHttpStatus::new).map_err(|_| {
                    SqliteError::decode_message(format!(
                        "crawl state http status out of range: {status}"
                    ))
                })
            })
            .transpose()?;
        let error = self
            .last_error_kind
            .as_deref()
            .map(codec::decode_crawl_state_error_kind)
            .transpose()?
            .map(|kind| CrawlStateError { kind });
        let failure_streak = u64::try_from(self.failure_streak).map_err(|_| {
            SqliteError::decode_message(format!(
                "crawl state failure streak must be non-negative: {}",
                self.failure_streak
            ))
        })?;

        let last = LastCrawlResult {
            started_at: self.last_started_at,
            finished_at: self.last_finished_at,
            http_status,
            error,
            retry_after: self.retry_after,
        };

        Ok(CrawlState {
            feed_url: feed_url.clone(),
            last,
            health: CrawlHealth {
                failure_streak: FailureStreak::new(failure_streak),
            },
            conditional: FeedConditionalFetch {
                etag: self.etag,
                last_modified: self.last_modified,
            },
        })
    }
}

fn encode_u64(value: u64, field: &'static str) -> SqliteResult<i64> {
    i64::try_from(value)
        .map_err(|_| SqliteError::decode_message(format!("{field} exceeds SQLite INTEGER range")))
}

impl CrawlStateStore for SqliteRegistryTx<'_> {
    async fn load_crawl_state(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<CrawlState>> {
        load(&mut self.tx, feed_url).await.db()
    }

    async fn upsert_crawl_state(
        &mut self,
        command: UpsertCrawlStateCommand,
    ) -> RegistryDbResult<()> {
        upsert(&mut self.tx, command).await.db()
    }
}

#[cfg(test)]
mod tests;
