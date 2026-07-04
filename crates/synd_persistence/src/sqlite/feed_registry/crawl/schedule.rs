use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CrawlScheduleStore, RegistryDbResult,
    crawl::schedule::{
        CompleteDispatchCommand, CrawlSchedule, DispatchCandidate, DueReason, ScheduleSyncEntry,
        ScheduledCrawlTarget, ScheduledCrawlTargetState, UpsertCrawlScheduleCommand,
    },
};

use super::super::{
    SqliteRegistryTx, codec,
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    feed_endpoint,
};

async fn load_schedule_sync_entry(
    tx: &mut Transaction<'_, Sqlite>,
    feed_url: &FeedUrl,
) -> SqliteResult<Option<ScheduleSyncEntry>> {
    let row = sqlx::query_as::<_, ScheduleSyncDbRow>(
        r#"
            SELECT
                e.url AS feed_url,
                ct.state AS target_state,
                ct.effective_policy_json AS target_policy_json,
                ct.updated_at AS target_updated_at,
                cs.target_updated_at AS schedule_target_updated_at,
                cs.next_crawl_after AS schedule_next_crawl_after,
                cs.due_reason AS schedule_due_reason,
                cs.dispatched_at AS schedule_dispatched_at,
                cs.created_at AS schedule_created_at,
                cs.updated_at AS schedule_updated_at
            FROM feed_endpoint AS e
            INNER JOIN crawl_target AS ct
                ON ct.feed_endpoint_pk = e.pk
            LEFT JOIN crawl_schedule AS cs
                ON cs.feed_endpoint_pk = e.pk
            WHERE e.url = ?
            "#,
    )
    .bind(feed_url.as_str())
    .fetch_optional(&mut **tx)
    .await?;

    row.map(ScheduleSyncDbRow::into_entry).transpose()
}

async fn upsert(
    tx: &mut Transaction<'_, Sqlite>,
    schedule: UpsertCrawlScheduleCommand,
) -> SqliteResult<()> {
    let feed_endpoint_pk = feed_endpoint::resolve_pk(tx, &schedule.feed_url).await?;

    sqlx::query(
        r#"
            INSERT INTO crawl_schedule (
                feed_endpoint_pk,
                target_updated_at,
                next_crawl_after,
                due_reason,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(feed_endpoint_pk) DO UPDATE SET
                target_updated_at = excluded.target_updated_at,
                next_crawl_after = excluded.next_crawl_after,
                due_reason = excluded.due_reason,
                updated_at = excluded.updated_at
            "#,
    )
    .bind(feed_endpoint_pk)
    .bind(schedule.target_updated_at)
    .bind(schedule.next_crawl_after)
    .bind(schedule.due_reason.as_str())
    .bind(schedule.synced_at)
    .bind(schedule.synced_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn complete_dispatch(
    tx: &mut Transaction<'_, Sqlite>,
    command: CompleteDispatchCommand,
) -> SqliteResult<()> {
    let feed_endpoint_pk = feed_endpoint::resolve_pk(tx, &command.feed_url).await?;

    sqlx::query(
        r#"
            INSERT INTO crawl_schedule (
                feed_endpoint_pk,
                target_updated_at,
                next_crawl_after,
                due_reason,
                dispatched_at,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, NULL, ?, ?)
            ON CONFLICT(feed_endpoint_pk) DO UPDATE SET
                target_updated_at = excluded.target_updated_at,
                next_crawl_after = excluded.next_crawl_after,
                due_reason = excluded.due_reason,
                dispatched_at = NULL,
                updated_at = excluded.updated_at
            "#,
    )
    .bind(feed_endpoint_pk)
    .bind(command.target_updated_at)
    .bind(command.next_crawl_after)
    .bind(command.due_reason.as_str())
    .bind(command.synced_at)
    .bind(command.synced_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn list_dispatchable(
    tx: &mut Transaction<'_, Sqlite>,
    now: DateTime<Utc>,
    stale_before: DateTime<Utc>,
    limit: usize,
) -> SqliteResult<Vec<DispatchCandidate>> {
    let limit = i64::try_from(limit).map_err(|_| {
        SqliteError::decode_message("dispatchable limit exceeds SQLite INTEGER range")
    })?;
    let rows = sqlx::query_as::<_, DispatchCandidateDbRow>(
        r#"
            SELECT
                e.url AS feed_url,
                COALESCE(cs.next_crawl_after, ?) AS due_at,
                cs.due_reason AS due_reason
            FROM crawl_schedule AS cs
            INNER JOIN crawl_target AS ct
                ON ct.feed_endpoint_pk = cs.feed_endpoint_pk
            INNER JOIN feed_endpoint AS e
                ON e.pk = cs.feed_endpoint_pk
            WHERE ct.state = ?
              AND (
                (cs.dispatched_at IS NULL
                    AND cs.next_crawl_after IS NOT NULL
                    AND cs.next_crawl_after <= ?)
                OR (cs.dispatched_at IS NOT NULL AND cs.dispatched_at <= ?)
              )
            ORDER BY
                CASE cs.due_reason
                    WHEN 'manual' THEN 0
                    WHEN 'retry' THEN 1
                    ELSE 2
                END,
                due_at,
                cs.feed_endpoint_pk
            LIMIT ?
            "#,
    )
    .bind(now)
    .bind(ScheduledTargetStateDb::ACTIVE)
    .bind(now)
    .bind(stale_before)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;

    rows.into_iter()
        .map(DispatchCandidateDbRow::into_candidate)
        .collect()
}

async fn mark_dispatched(
    tx: &mut Transaction<'_, Sqlite>,
    feed_urls: &[FeedUrl],
    dispatched_at: DateTime<Utc>,
) -> SqliteResult<()> {
    for feed_url in feed_urls {
        sqlx::query(
            r#"
                UPDATE crawl_schedule
                SET dispatched_at = ?, updated_at = ?
                WHERE feed_endpoint_pk = (SELECT pk FROM feed_endpoint WHERE url = ?)
                "#,
        )
        .bind(dispatched_at)
        .bind(dispatched_at)
        .bind(feed_url.as_str())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn next_dispatch_at(
    tx: &mut Transaction<'_, Sqlite>,
    now: DateTime<Utc>,
    stale_timeout: std::time::Duration,
) -> SqliteResult<Option<DateTime<Utc>>> {
    let next_due = sqlx::query_as::<_, NextInstantDbRow>(
        r#"
            SELECT
                MIN(cs.next_crawl_after) AS at
            FROM crawl_schedule AS cs
            INNER JOIN crawl_target AS ct
                ON ct.feed_endpoint_pk = cs.feed_endpoint_pk
            WHERE cs.dispatched_at IS NULL
              AND cs.next_crawl_after IS NOT NULL
              AND cs.next_crawl_after > ?
              AND ct.state = ?
            "#,
    )
    .bind(now)
    .bind(ScheduledTargetStateDb::ACTIVE)
    .fetch_one(&mut **tx)
    .await?
    .at;

    let earliest_dispatched = sqlx::query_as::<_, NextInstantDbRow>(
        r#"
            SELECT
                MIN(cs.dispatched_at) AS at
            FROM crawl_schedule AS cs
            INNER JOIN crawl_target AS ct
                ON ct.feed_endpoint_pk = cs.feed_endpoint_pk
            WHERE cs.dispatched_at IS NOT NULL
              AND ct.state = ?
            "#,
    )
    .bind(ScheduledTargetStateDb::ACTIVE)
    .fetch_one(&mut **tx)
    .await?
    .at;

    let stale_deadline = earliest_dispatched.map(|dispatched_at| {
        chrono::Duration::from_std(stale_timeout)
            .map_or(dispatched_at, |timeout| dispatched_at + timeout)
    });

    Ok(match (next_due, stale_deadline) {
        (Some(due), Some(stale)) => Some(due.min(stale)),
        (next, stale) => next.or(stale),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScheduledTargetStateDb {
    Active,
    Inactive,
}

impl ScheduledTargetStateDb {
    const ACTIVE: &'static str = "active";
    const INACTIVE: &'static str = "inactive";
}

impl FromStr for ScheduledTargetStateDb {
    type Err = SqliteError;

    fn from_str(state: &str) -> Result<Self, Self::Err> {
        match state {
            Self::ACTIVE => Ok(Self::Active),
            Self::INACTIVE => Ok(Self::Inactive),
            state => Err(SqliteError::decode_message(format!(
                "unknown crawl target state: {state}"
            ))),
        }
    }
}

impl fmt::Display for ScheduledTargetStateDb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Active => f.write_str(Self::ACTIVE),
            Self::Inactive => f.write_str(Self::INACTIVE),
        }
    }
}

#[derive(sqlx::FromRow)]
struct ScheduleSyncDbRow {
    feed_url: String,
    target_state: String,
    target_policy_json: Option<String>,
    target_updated_at: DateTime<Utc>,
    schedule_target_updated_at: Option<DateTime<Utc>>,
    schedule_next_crawl_after: Option<DateTime<Utc>>,
    schedule_due_reason: Option<String>,
    schedule_dispatched_at: Option<DateTime<Utc>>,
    schedule_created_at: Option<DateTime<Utc>>,
    schedule_updated_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct DispatchCandidateDbRow {
    feed_url: String,
    due_at: DateTime<Utc>,
    due_reason: String,
}

#[derive(sqlx::FromRow)]
struct NextInstantDbRow {
    at: Option<DateTime<Utc>>,
}

impl DispatchCandidateDbRow {
    fn into_candidate(self) -> SqliteResult<DispatchCandidate> {
        Ok(DispatchCandidate {
            feed_url: FeedUrl::parse(&self.feed_url).decode()?,
            due_at: self.due_at,
            due_reason: self.due_reason.parse::<DueReason>().decode()?,
        })
    }
}

impl ScheduleSyncDbRow {
    fn into_entry(self) -> SqliteResult<ScheduleSyncEntry> {
        let feed_url = FeedUrl::parse(&self.feed_url).decode()?;
        let state = match self.target_state.parse()? {
            ScheduledTargetStateDb::Active => {
                let policy_json = self.target_policy_json.ok_or_else(|| {
                    SqliteError::decode_message("active crawl target requires an effective policy")
                })?;
                ScheduledCrawlTargetState::Active {
                    polling: codec::decode_crawl_policy_json(&policy_json)?.polling,
                }
            }
            ScheduledTargetStateDb::Inactive => ScheduledCrawlTargetState::Inactive,
        };
        let target = ScheduledCrawlTarget::new(feed_url.clone(), self.target_updated_at, state);
        let schedule = match (
            self.schedule_target_updated_at,
            self.schedule_due_reason,
            self.schedule_created_at,
            self.schedule_updated_at,
        ) {
            (Some(target_updated_at), Some(due_reason), Some(created_at), Some(updated_at)) => {
                Some(
                    CrawlSchedule::builder()
                        .feed_url(feed_url)
                        .target_updated_at(target_updated_at)
                        .maybe_next_crawl_after(self.schedule_next_crawl_after)
                        .due_reason(due_reason.parse::<DueReason>().decode()?)
                        .maybe_dispatched_at(self.schedule_dispatched_at)
                        .created_at(created_at)
                        .updated_at(updated_at)
                        .build(),
                )
            }
            (None, None, None, None) => None,
            _ => {
                return Err(SqliteError::decode_message(format!(
                    "incomplete crawl schedule row for {}",
                    feed_url.as_str()
                )));
            }
        };

        Ok(ScheduleSyncEntry::new(target, schedule))
    }
}

impl CrawlScheduleStore for SqliteRegistryTx<'_> {
    async fn load_schedule_sync_entry(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Option<ScheduleSyncEntry>> {
        load_schedule_sync_entry(&mut self.tx, feed_url).await.db()
    }

    async fn upsert_schedule(
        &mut self,
        schedule: UpsertCrawlScheduleCommand,
    ) -> RegistryDbResult<()> {
        upsert(&mut self.tx, schedule).await.db()
    }

    async fn complete_dispatch(
        &mut self,
        command: CompleteDispatchCommand,
    ) -> RegistryDbResult<()> {
        complete_dispatch(&mut self.tx, command).await.db()
    }

    async fn list_dispatchable(
        &mut self,
        now: DateTime<Utc>,
        stale_before: DateTime<Utc>,
        limit: usize,
    ) -> RegistryDbResult<Vec<DispatchCandidate>> {
        list_dispatchable(&mut self.tx, now, stale_before, limit)
            .await
            .db()
    }

    async fn mark_dispatched(
        &mut self,
        feed_urls: &[FeedUrl],
        dispatched_at: DateTime<Utc>,
    ) -> RegistryDbResult<()> {
        mark_dispatched(&mut self.tx, feed_urls, dispatched_at)
            .await
            .db()
    }

    async fn next_dispatch_at(
        &mut self,
        now: DateTime<Utc>,
        stale_timeout: std::time::Duration,
    ) -> RegistryDbResult<Option<DateTime<Utc>>> {
        next_dispatch_at(&mut self.tx, now, stale_timeout)
            .await
            .db()
    }
}

#[cfg(test)]
mod tests;
