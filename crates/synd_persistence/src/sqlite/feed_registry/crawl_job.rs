use chrono::{DateTime, Utc};
use sqlx::{Row, Sqlite, Transaction};
use synd_feed::types::FeedUrl;
use synd_registry::{
    RegistryDbError, RegistryDbResult,
    crawl::job::{
        CrawlJob, CrawlJobId, CrawlJobState, CrawlQueueSnapshot, EnqueueJob, EnqueueJobResult,
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

    pub(super) async fn queue_snapshot(&mut self) -> RegistryDbResult<CrawlQueueSnapshot> {
        let row = sqlx::query(
            r#"
            SELECT
                COUNT(*) FILTER (WHERE state = 'pending') AS pending_count,
                COUNT(*) FILTER (WHERE state = 'running') AS running_count
            FROM crawl_job
            "#,
        )
        .fetch_one(&mut **self.tx)
        .await
        .map_err(RegistryDbError::internal)?;

        Ok(CrawlQueueSnapshot::new(
            row.try_get::<i64, _>("pending_count")
                .map_err(RegistryDbError::internal)?
                .try_into()
                .map_err(RegistryDbError::internal)?,
            row.try_get::<i64, _>("running_count")
                .map_err(RegistryDbError::internal)?
                .try_into()
                .map_err(RegistryDbError::internal)?,
        ))
    }

    pub(super) async fn enqueue(&mut self, job: EnqueueJob) -> RegistryDbResult<EnqueueJobResult> {
        let feed_endpoint_pk = {
            let mut feed_endpoint = FeedEndpointTable::new(&mut *self.tx);
            feed_endpoint.resolve_pk(&job.feed_url).await?
        };

        if self.active_job_exists(feed_endpoint_pk).await? {
            return Ok(EnqueueJobResult::AlreadyActive);
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

        Ok(EnqueueJobResult::Enqueued(row.try_into()?))
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
