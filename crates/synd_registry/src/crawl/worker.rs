use std::{sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use synd_feed::feed::service::{FeedFetchRequest, FetchFeed};
use synd_support::time::Clock;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::{
    crawl::{
        completion::{CrawlCompletionRecord, CrawlCompletionRecorder},
        job::{
            ClaimCrawlJobCommand, ClaimCrawlJobOutcome, CrawlJob, CrawlJobId, CrawlJobQueueLane,
            CrawlJobTrigger,
        },
        result::CrawlState,
    },
    db::{BlobStore, CommitTx, CrawlJobQueue, CrawlResultStore, FeedRegistryDb},
    event::{
        CrawlJobEnqueuedEvent, CrawlJobStartedEvent, EventInterests, EventJournal,
        EventJournalAppend, EventRecorder, EventWakePublisher, EventWorker, RecordedEvents,
        RegistryEvent, Trigger, WorkerHandle, WorkerId, WorkerResult, spawn_event_loop,
    },
};

const CLAIM_LANES: [CrawlJobQueueLane; 3] = [
    CrawlJobQueueLane::Manual,
    CrawlJobQueueLane::Default,
    CrawlJobQueueLane::Retry,
];

/// Runtime configuration for the crawl job worker pool.
#[derive(Debug, Clone, Copy)]
pub struct CrawlWorkerPoolConfig {
    pub max_running_jobs: usize,
    pub manual_queue: CrawlWorkerQueueConfig,
    pub default_queue: CrawlWorkerQueueConfig,
    pub retry_queue: CrawlWorkerQueueConfig,
    pub fetch: CrawlWorkerFetchConfig,
}

impl Default for CrawlWorkerPoolConfig {
    fn default() -> Self {
        Self {
            max_running_jobs: 4,
            manual_queue: CrawlWorkerQueueConfig {
                max_running_jobs: 2,
            },
            default_queue: CrawlWorkerQueueConfig {
                max_running_jobs: 4,
            },
            retry_queue: CrawlWorkerQueueConfig {
                max_running_jobs: 1,
            },
            fetch: CrawlWorkerFetchConfig::default(),
        }
    }
}

/// Queue-local crawl worker pool configuration.
#[derive(Debug, Clone, Copy)]
pub struct CrawlWorkerQueueConfig {
    pub max_running_jobs: usize,
}

/// HTTP fetch configuration used by crawl workers.
#[derive(Debug, Clone, Copy)]
pub struct CrawlWorkerFetchConfig {
    pub user_agent: &'static str,
    pub max_body_bytes: usize,
}

impl Default for CrawlWorkerFetchConfig {
    fn default() -> Self {
        Self {
            user_agent: "syndicationd",
            max_body_bytes: 10 * 1024 * 1024,
        }
    }
}

/// Claims and runs durable crawl jobs.
pub(crate) struct CrawlWorkerPool<S, F> {
    db: S,
    fetcher: F,
    wake_publisher: EventWakePublisher,
    ct: CancellationToken,
    capacity: CrawlWorkerCapacity,
    clock: Arc<dyn Clock>,
}

impl<S, F> CrawlWorkerPool<S, F> {
    pub fn new(
        db: S,
        fetcher: F,
        wake_publisher: EventWakePublisher,
        config: CrawlWorkerPoolConfig,
        ct: CancellationToken,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            db,
            fetcher,
            wake_publisher,
            ct,
            capacity: CrawlWorkerCapacity::new(config),
            clock,
        }
    }
}

impl<S, F> CrawlWorkerPool<S, F>
where
    S: FeedRegistryDb,
    F: FetchFeed + Clone + Send + Sync + 'static,
    for<'tx> S::Tx<'tx>:
        BlobStore + CrawlResultStore + CrawlJobQueue + EventJournalAppend + EventJournal + Send,
{
    async fn poll_and_dispatch(&mut self) -> WorkerResult<RecordedEvents> {
        let mut recorded = RecordedEvents::empty();
        let mut no_claimable_lanes = Vec::new();

        while self
            .try_start_next_job(&mut no_claimable_lanes, &mut recorded)
            .await?
        {}

        Ok(recorded)
    }

    async fn try_start_next_job(
        &mut self,
        no_claimable_lanes: &mut Vec<CrawlJobQueueLane>,
        recorded: &mut RecordedEvents,
    ) -> WorkerResult<bool> {
        for lane in CLAIM_LANES {
            if no_claimable_lanes.contains(&lane) {
                continue;
            }

            let Some(slot) = self.capacity.try_reserve(lane) else {
                continue;
            };

            let (outcome, poll_recorded) = self.poll_queue(lane).await?;
            recorded.extend(poll_recorded);

            match outcome {
                ClaimCrawlJobOutcome::Claimed(job) => {
                    self.start_worker(job, slot);
                    return Ok(true);
                }
                ClaimCrawlJobOutcome::NoClaimableJob => {
                    no_claimable_lanes.push(lane);
                    drop(slot);
                }
            }
        }

        Ok(false)
    }

    async fn poll_queue(
        &mut self,
        lane: CrawlJobQueueLane,
    ) -> WorkerResult<(ClaimCrawlJobOutcome, RecordedEvents)> {
        let mut tx = self.db.begin().await?;
        let mut recorded = RecordedEvents::empty();
        let outcome = tx
            .claim_job(ClaimCrawlJobCommand::new(lane, self.clock.now()))
            .await?;
        let produced: Vec<crate::event::Event> = match &outcome {
            ClaimCrawlJobOutcome::Claimed(job) => {
                vec![CrawlJobStartedEvent::from(job.clone()).into()]
            }
            ClaimCrawlJobOutcome::NoClaimableJob => Vec::new(),
        };
        EventRecorder::new(&mut tx, &mut recorded, self.clock.as_ref())
            .record_all(produced)
            .await?;
        tx.commit().await?;

        Ok((outcome, recorded))
    }

    fn start_worker(&self, job: CrawlJob, slot: CrawlWorkerSlot) {
        let db = self.db.clone();
        let fetcher = self.fetcher.clone();
        let wake_publisher = self.wake_publisher.clone();
        let worker_ct = self.ct.child_token();
        let clock = Arc::clone(&self.clock);
        tokio::spawn(async move {
            info!(
                worker = WorkerId::CrawlWorkerPool.as_str(),
                job_id = %job.job_id,
                feed_url = job.feed_url.as_str(),
                queue = slot.lane().as_str(),
                trigger = job.trigger.as_str(),
                "crawl job started"
            );

            let lane = slot.lane();
            let worker = CrawlWorker::new(db, fetcher, wake_publisher, worker_ct, clock);
            if let Err(err) = worker.run(job, lane).await {
                error!(
                    worker = WorkerId::CrawlWorkerPool.as_str(),
                    queue = lane.as_str(),
                    error = %err,
                    "crawl worker failed job"
                );
            }
            drop(slot);
        });
    }
}

impl<S, F> EventWorker for CrawlWorkerPool<S, F>
where
    S: FeedRegistryDb,
    F: FetchFeed + Clone + Send + Sync + 'static,
    for<'tx> S::Tx<'tx>:
        BlobStore + CrawlResultStore + CrawlJobQueue + EventJournalAppend + EventJournal + Send,
{
    fn id(&self) -> WorkerId {
        WorkerId::CrawlWorkerPool
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([CrawlJobEnqueuedEvent::TYPE])
    }

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<RecordedEvents> {
        self.poll_and_dispatch().await
    }
}

struct CrawlWorker<S, F> {
    db: S,
    fetcher: F,
    wake_publisher: EventWakePublisher,
    ct: CancellationToken,
    clock: Arc<dyn Clock>,
}

impl<S, F> CrawlWorker<S, F> {
    fn new(
        db: S,
        fetcher: F,
        wake_publisher: EventWakePublisher,
        ct: CancellationToken,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            db,
            fetcher,
            wake_publisher,
            ct,
            clock,
        }
    }
}

impl<S, F> CrawlWorker<S, F>
where
    S: FeedRegistryDb,
    F: FetchFeed + Send + Sync,
    for<'tx> S::Tx<'tx>:
        BlobStore + CrawlResultStore + CrawlJobQueue + EventJournalAppend + EventJournal + Send,
{
    #[tracing::instrument(
        name = "registry.crawl.worker.run",
        skip_all,
        fields(
            worker = WorkerId::CrawlWorkerPool.as_str(),
            job_id = %job.job_id,
            queue = lane.as_str()
        )
    )]
    async fn run(self, job: CrawlJob, lane: CrawlJobQueueLane) -> WorkerResult<()> {
        let job_id = job.job_id.clone();
        let feed_url = job.feed_url.clone();
        let trigger = job.trigger;
        let started_at = job.updated_at;
        let previous_state = self.load_previous_state(&job).await?;
        let request = feed_fetch_request(&job, previous_state.as_ref());

        let outcome = tokio::select! {
            () = self.ct.cancelled() => {
                debug!(
                    worker = WorkerId::CrawlWorkerPool.as_str(),
                    job_id = %job.job_id,
                    queue = lane.as_str(),
                    "crawl worker cancelled before fetch completed"
                );
                return Ok(());
            }
            outcome = self.fetcher.fetch_feed(request) => outcome,
        };

        let finished_at = self.clock.now();
        let (completion, recorded) = self
            .record_completion(job, outcome, previous_state, finished_at)
            .await?;
        log_crawl_job_completed(
            &job_id,
            feed_url.as_str(),
            lane,
            trigger,
            (finished_at - started_at).num_milliseconds(),
            &completion,
        );
        self.wake_publisher.publish(recorded);
        Ok(())
    }

    async fn load_previous_state(&self, job: &CrawlJob) -> WorkerResult<Option<CrawlState>> {
        let mut tx = self.db.begin().await?;
        let state = tx.load_crawl_state(&job.feed_url).await?;
        tx.commit().await?;
        Ok(state)
    }

    async fn record_completion(
        &self,
        job: CrawlJob,
        outcome: synd_feed::feed::service::FeedFetchOutcome,
        previous_state: Option<CrawlState>,
        finished_at: DateTime<Utc>,
    ) -> WorkerResult<(CrawlCompletionRecord, RecordedEvents)> {
        let mut tx = self.db.begin().await?;
        let mut completion_events = RecordedEvents::empty();
        let (record, produced) = CrawlCompletionRecorder::new(&mut tx)
            .record(job, outcome, previous_state, finished_at)
            .await?;
        EventRecorder::new(&mut tx, &mut completion_events, self.clock.as_ref())
            .record_all(produced)
            .await?;
        tx.commit().await?;
        Ok((record, completion_events))
    }
}

fn log_crawl_job_completed(
    job_id: &CrawlJobId,
    feed_url: &str,
    lane: CrawlJobQueueLane,
    trigger: CrawlJobTrigger,
    duration_ms: i64,
    completion: &CrawlCompletionRecord,
) {
    match (completion.http_status, completion.error_kind) {
        (Some(status), Some(error_kind)) => {
            info!(
                job_id = %job_id,
                feed_url,
                queue = lane.as_str(),
                trigger = trigger.as_str(),
                outcome = completion.outcome.as_str(),
                http_status = status.as_u16(),
                error_kind,
                result_ref = completion.result_ref.pk(),
                failure_streak = completion.health.failure_streak.value(),
                duration_ms,
                "crawl job completed"
            );
        }
        (Some(status), None) => {
            info!(
                job_id = %job_id,
                feed_url,
                queue = lane.as_str(),
                trigger = trigger.as_str(),
                outcome = completion.outcome.as_str(),
                http_status = status.as_u16(),
                result_ref = completion.result_ref.pk(),
                failure_streak = completion.health.failure_streak.value(),
                duration_ms,
                "crawl job completed"
            );
        }
        (None, Some(error_kind)) => {
            info!(
                job_id = %job_id,
                feed_url,
                queue = lane.as_str(),
                trigger = trigger.as_str(),
                outcome = completion.outcome.as_str(),
                error_kind,
                result_ref = completion.result_ref.pk(),
                failure_streak = completion.health.failure_streak.value(),
                duration_ms,
                "crawl job completed"
            );
        }
        (None, None) => {
            info!(
                job_id = %job_id,
                feed_url,
                queue = lane.as_str(),
                trigger = trigger.as_str(),
                outcome = completion.outcome.as_str(),
                result_ref = completion.result_ref.pk(),
                failure_streak = completion.health.failure_streak.value(),
                duration_ms,
                "crawl job completed"
            );
        }
    }
}

fn feed_fetch_request(job: &CrawlJob, previous_state: Option<&CrawlState>) -> FeedFetchRequest {
    let conditional = previous_state
        .map(|state| state.conditional.clone())
        .unwrap_or_default();
    FeedFetchRequest::new(job.feed_url.clone()).with_conditional(conditional)
}

struct CrawlWorkerCapacity {
    global: Arc<Semaphore>,
    manual: Arc<Semaphore>,
    default: Arc<Semaphore>,
    retry: Arc<Semaphore>,
}

impl CrawlWorkerCapacity {
    fn new(config: CrawlWorkerPoolConfig) -> Self {
        Self {
            global: Arc::new(Semaphore::new(config.max_running_jobs)),
            manual: Arc::new(Semaphore::new(config.manual_queue.max_running_jobs)),
            default: Arc::new(Semaphore::new(config.default_queue.max_running_jobs)),
            retry: Arc::new(Semaphore::new(config.retry_queue.max_running_jobs)),
        }
    }

    fn try_reserve(&self, lane: CrawlJobQueueLane) -> Option<CrawlWorkerSlot> {
        let lane_capacity = self.lane_capacity(lane);
        if self.global.available_permits() == 0 || lane_capacity.available_permits() == 0 {
            return None;
        }

        let global_permit = Arc::clone(&self.global).try_acquire_owned().ok()?;
        let lane_permit = lane_capacity.try_acquire_owned().ok()?;

        Some(CrawlWorkerSlot {
            lane,
            _global_permit: global_permit,
            _lane_permit: lane_permit,
        })
    }

    fn lane_capacity(&self, lane: CrawlJobQueueLane) -> Arc<Semaphore> {
        match lane {
            CrawlJobQueueLane::Default => Arc::clone(&self.default),
            CrawlJobQueueLane::Manual => Arc::clone(&self.manual),
            CrawlJobQueueLane::Retry => Arc::clone(&self.retry),
        }
    }
}

struct CrawlWorkerSlot {
    lane: CrawlJobQueueLane,
    _global_permit: OwnedSemaphorePermit,
    _lane_permit: OwnedSemaphorePermit,
}

impl CrawlWorkerSlot {
    fn lane(&self) -> CrawlJobQueueLane {
        self.lane
    }
}

pub(crate) fn spawn_crawl_worker_pool<S, F>(
    db: S,
    fetcher: F,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    config: CrawlWorkerPoolConfig,
    ct: CancellationToken,
    clock: Arc<dyn Clock>,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    F: FetchFeed + Clone + Send + Sync + 'static,
    for<'tx> S::Tx<'tx>:
        BlobStore + CrawlResultStore + CrawlJobQueue + EventJournalAppend + EventJournal + Send,
{
    let pool = CrawlWorkerPool::new(
        db,
        fetcher,
        wake_publisher.clone(),
        config,
        ct.clone(),
        clock,
    );
    spawn_event_loop(pool, wake_publisher, poll_interval, ct)
}

#[cfg(test)]
mod tests {
    use super::{
        CLAIM_LANES, CrawlWorkerCapacity, CrawlWorkerFetchConfig, CrawlWorkerPoolConfig,
        CrawlWorkerQueueConfig,
    };
    use crate::crawl::job::CrawlJobQueueLane;

    mod capacity {
        use super::{CLAIM_LANES, CrawlJobQueueLane, CrawlWorkerCapacity, config};

        #[test]
        fn enforces_global_and_lane_capacity() {
            let capacity = CrawlWorkerCapacity::new(config(2, 1, 2, 1));

            let manual = capacity
                .try_reserve(CrawlJobQueueLane::Manual)
                .expect("manual slot should be available");
            assert!(capacity.try_reserve(CrawlJobQueueLane::Manual).is_none());

            let default = capacity
                .try_reserve(CrawlJobQueueLane::Default)
                .expect("default slot should be available");
            assert!(capacity.try_reserve(CrawlJobQueueLane::Retry).is_none());

            drop(manual);
            let retry = capacity
                .try_reserve(CrawlJobQueueLane::Retry)
                .expect("released global slot should be reusable");

            drop(default);
            drop(retry);
        }

        #[test]
        fn claim_lanes_are_fixed_priority_order() {
            assert_eq!(
                CLAIM_LANES,
                [
                    CrawlJobQueueLane::Manual,
                    CrawlJobQueueLane::Default,
                    CrawlJobQueueLane::Retry
                ]
            );
        }
    }

    fn config(
        max_running_jobs: usize,
        manual_max_running_jobs: usize,
        default_max_running_jobs: usize,
        retry_max_running_jobs: usize,
    ) -> CrawlWorkerPoolConfig {
        CrawlWorkerPoolConfig {
            max_running_jobs,
            manual_queue: CrawlWorkerQueueConfig {
                max_running_jobs: manual_max_running_jobs,
            },
            default_queue: CrawlWorkerQueueConfig {
                max_running_jobs: default_max_running_jobs,
            },
            retry_queue: CrawlWorkerQueueConfig {
                max_running_jobs: retry_max_running_jobs,
            },
            fetch: CrawlWorkerFetchConfig::default(),
        }
    }
}
