use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CrawlScheduleStore, RegistryDbResult,
    crawl::{
        job::{ActiveCrawlJob, CrawlJobId},
        schedule::{
            CrawlReadiness, CrawlSchedule, CrawlScheduleCandidate, ScheduledCrawlTarget,
            ScheduledCrawlTargetState, UpsertCrawlScheduleCommand,
        },
    },
};

use super::super::{
    SqliteRegistryTx, codec,
    error::{DecodeResultExt, IntoDbResult, SqliteError, SqliteResult},
    feed_endpoint,
};

async fn list_candidates(
    tx: &mut Transaction<'_, Sqlite>,
    now: DateTime<Utc>,
    limit: usize,
) -> SqliteResult<Vec<CrawlScheduleCandidate>> {
    let limit = i64::try_from(limit)
        .map_err(|_| SqliteError::decode_message("candidate limit exceeds SQLite INTEGER range"))?;
    let rows = sqlx::query_as::<_, CrawlScheduleCandidateRow>(
        r#"
            SELECT
                e.url AS feed_url,
                ct.state AS target_state,
                ct.effective_policy_json AS target_policy_json,
                ct.updated_at AS target_updated_at,
                cs.target_updated_at AS schedule_target_updated_at,
                cs.next_crawl_after AS schedule_next_crawl_after,
                cs.created_at AS schedule_created_at,
                cs.updated_at AS schedule_updated_at,
                active.job_id AS active_job_id,
                active.state AS active_job_state
            FROM crawl_target AS ct
            INNER JOIN feed_endpoint AS e
                ON e.pk = ct.feed_endpoint_pk
            LEFT JOIN crawl_schedule AS cs
                ON cs.feed_endpoint_pk = ct.feed_endpoint_pk
            LEFT JOIN crawl_job AS active
                ON active.feed_endpoint_pk = ct.feed_endpoint_pk
               AND active.state IN ('pending', 'running')
            WHERE cs.feed_endpoint_pk IS NULL
               OR cs.target_updated_at != ct.updated_at
               OR cs.next_crawl_after <= ?
            ORDER BY
                cs.next_crawl_after IS NULL,
                cs.next_crawl_after,
                ct.updated_at,
                ct.feed_endpoint_pk
            LIMIT ?
            "#,
    )
    .bind(now)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;

    rows.into_iter()
        .map(CrawlScheduleCandidateRow::into_candidate)
        .collect()
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
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(feed_endpoint_pk) DO UPDATE SET
                target_updated_at = excluded.target_updated_at,
                next_crawl_after = excluded.next_crawl_after,
                updated_at = excluded.updated_at
            "#,
    )
    .bind(feed_endpoint_pk)
    .bind(schedule.target_updated_at)
    .bind(schedule.next_crawl_after)
    .bind(schedule.reconciled_at)
    .bind(schedule.reconciled_at)
    .execute(&mut **tx)
    .await?;

    Ok(())
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
struct CrawlScheduleCandidateRow {
    feed_url: String,
    target_state: String,
    target_policy_json: Option<String>,
    target_updated_at: DateTime<Utc>,
    schedule_target_updated_at: Option<DateTime<Utc>>,
    schedule_next_crawl_after: Option<DateTime<Utc>>,
    schedule_created_at: Option<DateTime<Utc>>,
    schedule_updated_at: Option<DateTime<Utc>>,
    active_job_id: Option<String>,
    active_job_state: Option<String>,
}

impl CrawlScheduleCandidateRow {
    fn into_candidate(self) -> SqliteResult<CrawlScheduleCandidate> {
        let feed_url = FeedUrl::parse(&self.feed_url).decode()?;
        let state = match self.target_state.parse()? {
            ScheduledTargetStateDb::Active => {
                let policy_json = self.target_policy_json.ok_or_else(|| {
                    SqliteError::decode_message("active crawl target requires an effective policy")
                })?;
                ScheduledCrawlTargetState::Active {
                    policy: codec::decode_crawl_policy_json(&policy_json)?,
                }
            }
            ScheduledTargetStateDb::Inactive => ScheduledCrawlTargetState::Inactive,
        };
        let target = ScheduledCrawlTarget::new(feed_url.clone(), self.target_updated_at, state);
        let schedule = match (
            self.schedule_target_updated_at,
            self.schedule_created_at,
            self.schedule_updated_at,
        ) {
            (Some(target_updated_at), Some(created_at), Some(updated_at)) => {
                Some(CrawlSchedule::new(
                    feed_url.clone(),
                    target_updated_at,
                    self.schedule_next_crawl_after,
                    created_at,
                    updated_at,
                ))
            }
            (None, None, None) => None,
            _ => {
                return Err(SqliteError::decode_message(format!(
                    "incomplete crawl schedule row for {}",
                    feed_url.as_str()
                )));
            }
        };
        let active_job = match (self.active_job_id, self.active_job_state) {
            (Some(job_id), Some(state)) => Some(ActiveCrawlJob::new(
                CrawlJobId::new(job_id),
                state.parse().decode()?,
            )),
            (None, None) => None,
            _ => {
                return Err(SqliteError::decode_message(format!(
                    "incomplete active crawl job row for {}",
                    feed_url.as_str()
                )));
            }
        };

        Ok(CrawlScheduleCandidate::new(
            target,
            schedule,
            active_job,
            CrawlReadiness::ready(),
        ))
    }
}

impl CrawlScheduleStore for SqliteRegistryTx<'_> {
    async fn list_candidates(
        &mut self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> RegistryDbResult<Vec<CrawlScheduleCandidate>> {
        list_candidates(&mut self.tx, now, limit).await.db()
    }

    async fn upsert_schedule(
        &mut self,
        schedule: UpsertCrawlScheduleCommand,
    ) -> RegistryDbResult<()> {
        upsert(&mut self.tx, schedule).await.db()
    }
}

#[cfg(test)]
mod tests;
