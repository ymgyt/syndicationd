use chrono::{DateTime, Utc};
use sqlx::{Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    CrawlJobQueueTx, RegistryDbResult,
    crawl::job::{
        ClaimCrawlJobCommand, ClaimCrawlJobOutcome, CrawlJob, CrawlJobId, CrawlJobState,
        EnqueueCrawlJobCommand, EnqueueCrawlJobOutcome, FinishCrawlJobCommand,
        FinishCrawlJobOutcome,
    },
};

use super::super::{
    SqliteRegistryTx,
    error::{DecodeResultExt, IntoDbResult, SqliteResult},
    feed_endpoint,
};

async fn enqueue(
    tx: &mut Transaction<'_, Sqlite>,
    job: EnqueueCrawlJobCommand,
) -> SqliteResult<EnqueueCrawlJobOutcome> {
    let feed_endpoint_pk = feed_endpoint::resolve_pk(tx, &job.feed_url).await?;

    if active_job_exists(tx, feed_endpoint_pk).await? {
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
    .fetch_one(&mut **tx)
    .await?;

    Ok(EnqueueCrawlJobOutcome::Enqueued(row.into_job()?))
}

async fn claim(
    tx: &mut Transaction<'_, Sqlite>,
    command: ClaimCrawlJobCommand,
) -> SqliteResult<ClaimCrawlJobOutcome> {
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
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Ok(ClaimCrawlJobOutcome::NoClaimableJob);
    };

    let feed_url = feed_endpoint::resolve_url(tx, row.feed_endpoint_pk).await?;

    Ok(ClaimCrawlJobOutcome::Claimed(row.into_job(feed_url)?))
}

async fn finish(
    tx: &mut Transaction<'_, Sqlite>,
    command: FinishCrawlJobCommand,
) -> SqliteResult<FinishCrawlJobOutcome> {
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
    .fetch_optional(&mut **tx)
    .await?;

    let Some(row) = row else {
        return Ok(FinishCrawlJobOutcome::NotRunning);
    };

    let feed_url = feed_endpoint::resolve_url(tx, row.feed_endpoint_pk).await?;

    Ok(FinishCrawlJobOutcome::Finished(row.into_job(feed_url)?))
}

async fn active_job_exists(
    tx: &mut Transaction<'_, Sqlite>,
    feed_endpoint_pk: i64,
) -> SqliteResult<bool> {
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
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.is_some())
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
    fn into_job(self, feed_url: FeedUrl) -> SqliteResult<CrawlJob> {
        Ok(CrawlJob::new(
            CrawlJobId::new(self.job_id),
            feed_url,
            self.state.parse().decode()?,
            self.trigger.parse().decode()?,
            self.queue.parse().decode()?,
            self.priority,
            self.run_after,
            self.created_at,
            self.updated_at,
        ))
    }
}

impl CrawlJobQueueTx for SqliteRegistryTx<'_> {
    async fn enqueue_job(
        &mut self,
        job: EnqueueCrawlJobCommand,
    ) -> RegistryDbResult<EnqueueCrawlJobOutcome> {
        enqueue(&mut self.tx, job).await.db()
    }

    async fn claim_job(
        &mut self,
        command: ClaimCrawlJobCommand,
    ) -> RegistryDbResult<ClaimCrawlJobOutcome> {
        claim(&mut self.tx, command).await.db()
    }

    async fn finish_job(
        &mut self,
        command: FinishCrawlJobCommand,
    ) -> RegistryDbResult<FinishCrawlJobOutcome> {
        finish(&mut self.tx, command).await.db()
    }
}

impl CrawlJobRow {
    fn into_job(self) -> SqliteResult<CrawlJob> {
        Ok(CrawlJob::new(
            CrawlJobId::new(self.job_id),
            FeedUrl::parse(&self.feed_url).decode()?,
            self.state.parse().decode()?,
            self.trigger.parse().decode()?,
            self.queue.parse().decode()?,
            self.priority,
            self.run_after,
            self.created_at,
            self.updated_at,
        ))
    }
}
