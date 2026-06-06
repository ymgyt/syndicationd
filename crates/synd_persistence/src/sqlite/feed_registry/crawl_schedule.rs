use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    RegistryDbError, RegistryDbResult,
    crawl::{
        job::{ActiveCrawlJob, CrawlJobId},
        schedule::{
            CrawlReadiness, CrawlSchedule, CrawlScheduleCandidate, ScheduledCrawlTarget,
            ScheduledCrawlTargetState, UpsertSchedule,
        },
    },
};

use super::{codec, feed_endpoint::FeedEndpointTable};

pub(super) struct CrawlScheduleTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> CrawlScheduleTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn list_candidates(
        &mut self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> RegistryDbResult<Vec<CrawlScheduleCandidate>> {
        let limit = i64::try_from(limit).map_err(RegistryDbError::internal)?;
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
        .fetch_all(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        rows.into_iter()
            .map(CrawlScheduleCandidate::try_from)
            .collect()
    }

    pub(super) async fn upsert(&mut self, schedule: UpsertSchedule) -> RegistryDbResult<()> {
        let feed_endpoint_pk = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_pk(&schedule.feed_url).await?
        };

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
        .execute(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(())
    }
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
    type Err = RegistryDbError;

    fn from_str(state: &str) -> Result<Self, Self::Err> {
        match state {
            Self::ACTIVE => Ok(Self::Active),
            Self::INACTIVE => Ok(Self::Inactive),
            state => Err(RegistryDbError::internal(anyhow::anyhow!(
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

impl TryFrom<CrawlScheduleCandidateRow> for CrawlScheduleCandidate {
    type Error = RegistryDbError;

    fn try_from(row: CrawlScheduleCandidateRow) -> Result<Self, Self::Error> {
        let feed_url = FeedUrl::parse(&row.feed_url).map_err(RegistryDbError::internal)?;
        let state = match row.target_state.parse()? {
            ScheduledTargetStateDb::Active => {
                let policy_json = row.target_policy_json.ok_or_else(|| {
                    RegistryDbError::internal(anyhow::anyhow!(
                        "active crawl target requires an effective policy"
                    ))
                })?;
                ScheduledCrawlTargetState::Active {
                    policy: codec::decode_crawl_policy_json(&policy_json)?,
                }
            }
            ScheduledTargetStateDb::Inactive => ScheduledCrawlTargetState::Inactive,
        };
        let target = ScheduledCrawlTarget::new(feed_url.clone(), row.target_updated_at, state);
        let schedule = match (
            row.schedule_target_updated_at,
            row.schedule_created_at,
            row.schedule_updated_at,
        ) {
            (Some(target_updated_at), Some(created_at), Some(updated_at)) => {
                Some(CrawlSchedule::new(
                    feed_url.clone(),
                    target_updated_at,
                    row.schedule_next_crawl_after,
                    created_at,
                    updated_at,
                ))
            }
            (None, None, None) => None,
            _ => {
                return Err(RegistryDbError::internal(anyhow::anyhow!(
                    "incomplete crawl schedule row for {}",
                    feed_url.as_str()
                )));
            }
        };
        let active_job = match (row.active_job_id, row.active_job_state) {
            (Some(job_id), Some(state)) => Some(ActiveCrawlJob::new(
                CrawlJobId::new(job_id),
                state.parse().map_err(RegistryDbError::internal)?,
            )),
            (None, None) => None,
            _ => {
                return Err(RegistryDbError::internal(anyhow::anyhow!(
                    "incomplete active crawl job row for {}",
                    feed_url.as_str()
                )));
            }
        };

        Ok(Self::new(
            target,
            schedule,
            active_job,
            CrawlReadiness::ready(),
        ))
    }
}
