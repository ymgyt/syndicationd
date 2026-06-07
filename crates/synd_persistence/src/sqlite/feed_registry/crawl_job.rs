use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    RegistryDbError, RegistryDbResult,
    crawl::job::{
        ClaimCrawlJobCommand, ClaimCrawlJobOutcome, CrawlJob, CrawlJobId, CrawlJobState,
        EnqueueCrawlJobCommand, EnqueueCrawlJobOutcome, FinishCrawlJobCommand,
        FinishCrawlJobOutcome,
    },
};

use super::feed_endpoint::FeedEndpointTable;

pub(super) struct CrawlJobTable<'tx, 'db> {
    tx: &'tx mut Transaction<'db, Sqlite>,
}

impl<'tx, 'db> CrawlJobTable<'tx, 'db> {
    pub(super) fn new(tx: &'tx mut Transaction<'db, Sqlite>) -> Self {
        Self { tx }
    }

    pub(super) async fn enqueue(
        &mut self,
        job: EnqueueCrawlJobCommand,
    ) -> RegistryDbResult<EnqueueCrawlJobOutcome> {
        let feed_endpoint_pk = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_pk(&job.feed_url).await?
        };

        if self.active_job_exists(feed_endpoint_pk).await? {
            return Ok(EnqueueCrawlJobOutcome::AlreadyActive);
        }

        let job_id = CrawlJobId::generate();
        let row = sqlx::query_as::<_, CrawlJobRow>(
            r#"
            INSERT INTO crawl_job (
                job_id,
                feed_endpoint_pk,
                state,
                trigger,
                queue,
                priority,
                run_after,
                created_at,
                updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            RETURNING
                job_id,
                ? AS feed_url,
                state,
                trigger,
                queue,
                priority,
                run_after,
                created_at,
                updated_at
            "#,
        )
        .bind(job_id.as_str())
        .bind(feed_endpoint_pk)
        .bind(CrawlJobState::Pending.as_str())
        .bind(job.trigger.as_str())
        .bind(job.queue.as_str())
        .bind(job.priority)
        .bind(job.run_after)
        .bind(job.enqueued_at)
        .bind(job.enqueued_at)
        .bind(job.feed_url.as_str())
        .fetch_one(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(EnqueueCrawlJobOutcome::Enqueued(row.try_into()?))
    }

    pub(super) async fn claim(
        &mut self,
        command: ClaimCrawlJobCommand,
    ) -> RegistryDbResult<ClaimCrawlJobOutcome> {
        let row = sqlx::query_as::<_, ClaimedCrawlJobRow>(
            r#"
            UPDATE crawl_job
            SET
                state = ?,
                updated_at = ?
            WHERE pk = (
                SELECT pk
                FROM crawl_job
                WHERE state = ?
                  AND queue = ?
                  AND run_after <= ?
                ORDER BY priority DESC, run_after ASC, pk ASC
                LIMIT 1
            )
            RETURNING
                job_id,
                feed_endpoint_pk,
                state,
                trigger,
                queue,
                priority,
                run_after,
                created_at,
                updated_at
            "#,
        )
        .bind(CrawlJobState::Running.as_str())
        .bind(command.claimed_at)
        .bind(CrawlJobState::Pending.as_str())
        .bind(command.queue.as_str())
        .bind(command.claimed_at)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Ok(ClaimCrawlJobOutcome::NoClaimableJob);
        };

        let feed_url = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_url(row.feed_endpoint_pk).await?
        };

        Ok(ClaimCrawlJobOutcome::Claimed(row.into_job(feed_url)?))
    }

    pub(super) async fn finish(
        &mut self,
        command: FinishCrawlJobCommand,
    ) -> RegistryDbResult<FinishCrawlJobOutcome> {
        let row = sqlx::query_as::<_, ClaimedCrawlJobRow>(
            r#"
            UPDATE crawl_job
            SET
                state = ?,
                updated_at = ?
            WHERE job_id = ?
              AND state = ?
            RETURNING
                job_id,
                feed_endpoint_pk,
                state,
                trigger,
                queue,
                priority,
                run_after,
                created_at,
                updated_at
            "#,
        )
        .bind(CrawlJobState::Finished.as_str())
        .bind(command.finished_at)
        .bind(command.job_id.as_str())
        .bind(CrawlJobState::Running.as_str())
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        let Some(row) = row else {
            return Ok(FinishCrawlJobOutcome::NotRunning);
        };

        let feed_url = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_url(row.feed_endpoint_pk).await?
        };

        Ok(FinishCrawlJobOutcome::Finished(row.into_job(feed_url)?))
    }

    async fn active_job_exists(&mut self, feed_endpoint_pk: i64) -> RegistryDbResult<bool> {
        let row = sqlx::query(
            r#"
            SELECT 1
            FROM crawl_job
            WHERE feed_endpoint_pk = ?
              AND state IN ('pending', 'running')
            LIMIT 1
            "#,
        )
        .bind(feed_endpoint_pk)
        .fetch_optional(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(row.is_some())
    }
}

#[derive(sqlx::FromRow)]
struct CrawlJobRow {
    job_id: String,
    feed_url: String,
    state: String,
    trigger: String,
    queue: String,
    priority: i64,
    run_after: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ClaimedCrawlJobRow {
    job_id: String,
    feed_endpoint_pk: i64,
    state: String,
    trigger: String,
    queue: String,
    priority: i64,
    run_after: DateTime<Utc>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl ClaimedCrawlJobRow {
    fn into_job(self, feed_url: FeedUrl) -> RegistryDbResult<CrawlJob> {
        Ok(CrawlJob::new(
            CrawlJobId::new(self.job_id),
            feed_url,
            self.state.parse().map_err(RegistryDbError::internal)?,
            self.trigger.parse().map_err(RegistryDbError::internal)?,
            self.queue.parse().map_err(RegistryDbError::internal)?,
            self.priority,
            self.run_after,
            self.created_at,
            self.updated_at,
        ))
    }
}

impl TryFrom<CrawlJobRow> for CrawlJob {
    type Error = RegistryDbError;

    fn try_from(row: CrawlJobRow) -> Result<Self, Self::Error> {
        Ok(CrawlJob::new(
            CrawlJobId::new(row.job_id),
            FeedUrl::parse(&row.feed_url).map_err(RegistryDbError::internal)?,
            row.state.parse().map_err(RegistryDbError::internal)?,
            row.trigger.parse().map_err(RegistryDbError::internal)?,
            row.queue.parse().map_err(RegistryDbError::internal)?,
            row.priority,
            row.run_after,
            row.created_at,
            row.updated_at,
        ))
    }
}
