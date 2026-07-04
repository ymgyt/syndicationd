use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CrawlScheduleStore, RegistryDbResult,
    crawl::schedule::{
        CrawlSchedule, ScheduleSyncEntry, ScheduledCrawlTarget, ScheduledCrawlTargetState,
        ScheduledDue, UpsertCrawlScheduleCommand,
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

async fn list_schedule_sync_entries(
    tx: &mut Transaction<'_, Sqlite>,
    limit: usize,
) -> SqliteResult<Vec<ScheduleSyncEntry>> {
    let limit = i64::try_from(limit).map_err(|_| {
        SqliteError::decode_message("schedule sync limit exceeds SQLite INTEGER range")
    })?;
    let rows = sqlx::query_as::<_, ScheduleSyncDbRow>(
        r#"
            SELECT
                e.url AS feed_url,
                ct.state AS target_state,
                ct.effective_policy_json AS target_policy_json,
                ct.updated_at AS target_updated_at,
                cs.target_updated_at AS schedule_target_updated_at,
                cs.next_crawl_after AS schedule_next_crawl_after,
                cs.created_at AS schedule_created_at,
                cs.updated_at AS schedule_updated_at
            FROM crawl_target AS ct
            INNER JOIN feed_endpoint AS e
                ON e.pk = ct.feed_endpoint_pk
            LEFT JOIN crawl_schedule AS cs
                ON cs.feed_endpoint_pk = ct.feed_endpoint_pk
            WHERE cs.feed_endpoint_pk IS NULL
               OR cs.target_updated_at != ct.updated_at
            ORDER BY
                ct.updated_at,
                ct.feed_endpoint_pk
            LIMIT ?
            "#,
    )
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;

    rows.into_iter()
        .map(ScheduleSyncDbRow::into_entry)
        .collect()
}

async fn list_scheduled_due(
    tx: &mut Transaction<'_, Sqlite>,
    now: DateTime<Utc>,
    limit: usize,
) -> SqliteResult<Vec<ScheduledDue>> {
    let limit = i64::try_from(limit).map_err(|_| {
        SqliteError::decode_message("scheduled due limit exceeds SQLite INTEGER range")
    })?;
    let rows = sqlx::query_as::<_, ScheduledDueDbRow>(
        r#"
            SELECT
                e.url AS feed_url,
                cs.next_crawl_after AS due_at
            FROM crawl_schedule AS cs
            INNER JOIN crawl_target AS ct
                ON ct.feed_endpoint_pk = cs.feed_endpoint_pk
            INNER JOIN feed_endpoint AS e
                ON e.pk = cs.feed_endpoint_pk
            WHERE cs.next_crawl_after IS NOT NULL
              AND cs.next_crawl_after <= ?
              AND ct.state = ?
            ORDER BY
                cs.next_crawl_after,
                cs.feed_endpoint_pk
            LIMIT ?
            "#,
    )
    .bind(now)
    .bind(ScheduledTargetStateDb::ACTIVE)
    .bind(limit)
    .fetch_all(&mut **tx)
    .await?;

    rows.into_iter()
        .map(ScheduledDueDbRow::into_scheduled_due)
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
struct ScheduleSyncDbRow {
    feed_url: String,
    target_state: String,
    target_policy_json: Option<String>,
    target_updated_at: DateTime<Utc>,
    schedule_target_updated_at: Option<DateTime<Utc>>,
    schedule_next_crawl_after: Option<DateTime<Utc>>,
    schedule_created_at: Option<DateTime<Utc>>,
    schedule_updated_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct ScheduledDueDbRow {
    feed_url: String,
    due_at: DateTime<Utc>,
}

impl ScheduledDueDbRow {
    fn into_scheduled_due(self) -> SqliteResult<ScheduledDue> {
        Ok(ScheduledDue::new(
            FeedUrl::parse(&self.feed_url).decode()?,
            self.due_at,
        ))
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
                    feed_url,
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

    async fn list_schedule_sync_entries(
        &mut self,
        limit: usize,
    ) -> RegistryDbResult<Vec<ScheduleSyncEntry>> {
        list_schedule_sync_entries(&mut self.tx, limit).await.db()
    }

    async fn list_scheduled_due(
        &mut self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> RegistryDbResult<Vec<ScheduledDue>> {
        list_scheduled_due(&mut self.tx, now, limit).await.db()
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
