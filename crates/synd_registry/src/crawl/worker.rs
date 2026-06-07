use std::{sync::Arc, time::Duration};

use chrono::Utc;
use synd_feed::feed::service::{FeedFetchRequest, FetchFeed};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    crawl::{
        completion::CrawlCompletionRecorder,
        job::{ClaimCrawlJobCommand, ClaimCrawlJobOutcome, CrawlJob, CrawlJobQueueLane},
        queue::CrawlJobQueue,
        result::CrawlState,
    },
    db::{BlobStoreTx, CommitTx, CrawlCompletionTx, CrawlJobQueueTx, FeedRegistryDb},
    event::{
        CrawlEventKind, EventInterests, EventWake, EventWakePublisher, EventWakeRecvError,
        JournalTx, RecordedEvents, Trigger, WorkerHandle, WorkerId, WorkerResult,
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
    wake: EventWake,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    capacity: CrawlWorkerCapacity,
}

impl<S, F> CrawlWorkerPool<S, F> {
    pub fn new(
        db: S,
        fetcher: F,
        wake_publisher: EventWakePublisher,
        poll_interval: Duration,
        config: CrawlWorkerPoolConfig,
        ct: CancellationToken,
    ) -> Self {
        let wake = EventWake::new(wake_publisher.clone());
        Self {
            db,
            fetcher,
            wake,
            wake_publisher,
            poll_interval,
            ct,
            capacity: CrawlWorkerCapacity::new(config),
        }
    }
}

impl<S, F> CrawlWorkerPool<S, F>
where
    S: FeedRegistryDb,
    F: FetchFeed + Clone + Send + Sync + 'static,
    for<'tx> S::Tx<'tx>: BlobStoreTx + CrawlCompletionTx + CrawlJobQueueTx + JournalTx + Send,
{
    pub fn spawn(self) -> WorkerHandle {
        WorkerHandle::new(WorkerId::CrawlWorkerPool, tokio::spawn(self.run()))
    }

    async fn run(mut self) {
        debug!(
            worker = WorkerId::CrawlWorkerPool.as_str(),
            "crawl worker pool started"
        );
        self.process(Trigger::Startup).await;

        let mut interval = tokio::time::interval(self.poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;

        loop {
            tokio::select! {
                () = self.ct.cancelled() => break,
                wake = self.wake.recv() => {
                    match wake {
                        Ok(recorded) if Self::interests().matches_any(recorded.kinds()) => {
                            self.process(Trigger::Wake).await;
                        }
                        Ok(_) => {}
                        Err(EventWakeRecvError::Lagged(skipped)) => {
                            warn!(
                                worker = WorkerId::CrawlWorkerPool.as_str(),
                                skipped,
                                "crawl worker pool wake lagged"
                            );
                            self.process(Trigger::WakeLagged).await;
                        }
                        Err(EventWakeRecvError::Closed) => {
                            warn!(
                                worker = WorkerId::CrawlWorkerPool.as_str(),
                                "crawl worker pool wake channel closed"
                            );
                            break;
                        }
                    }
                }
                _ = interval.tick() => {
                    self.process(Trigger::Poll).await;
                }
            }
        }

        debug!(
            worker = WorkerId::CrawlWorkerPool.as_str(),
            "crawl worker pool stopped"
        );
    }

    fn interests() -> EventInterests {
        EventInterests::new([CrawlEventKind::JobEnqueued.into()])
    }

    #[tracing::instrument(
        name = "registry.crawl.worker_pool.process",
        skip_all,
        fields(
            worker = WorkerId::CrawlWorkerPool.as_str(),
            trigger = trigger.as_str()
        )
    )]
    async fn process(&mut self, trigger: Trigger) {
        let mut recorded = RecordedEvents::empty();

        match self.poll_and_dispatch(&mut recorded).await {
            Ok(()) => {
                self.wake.publish(recorded);
                debug!(
                    worker = WorkerId::CrawlWorkerPool.as_str(),
                    trigger = trigger.as_str(),
                    "crawl worker pool poll completed"
                );
            }
            Err(err) => {
                error!(
                    worker = WorkerId::CrawlWorkerPool.as_str(),
                    trigger = trigger.as_str(),
                    error = %err,
                    "crawl worker pool failed"
                );
            }
        }
    }

    async fn poll_and_dispatch(&mut self, recorded: &mut RecordedEvents) -> WorkerResult<()> {
        let mut no_claimable_lanes = Vec::new();

        while self
            .try_start_next_job(&mut no_claimable_lanes, recorded)
            .await?
        {}

        Ok(())
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
        let outcome = {
            let mut queue = CrawlJobQueue::new(&mut tx, &mut recorded);
            queue
                .claim(ClaimCrawlJobCommand::new(lane, Utc::now()))
                .await?
        };
        tx.commit().await?;

        Ok((outcome, recorded))
    }

    fn start_worker(&self, job: CrawlJob, slot: CrawlWorkerSlot) {
        let db = self.db.clone();
        let fetcher = self.fetcher.clone();
        let wake_publisher = self.wake_publisher.clone();
        let worker_ct = self.ct.child_token();
        tokio::spawn(async move {
            debug!(
                worker = WorkerId::CrawlWorkerPool.as_str(),
                job_id = %job.job_id,
                queue = slot.lane().as_str(),
                "crawl worker started job"
            );

            let lane = slot.lane();
            let worker = CrawlWorker::new(db, fetcher, wake_publisher, worker_ct);
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

struct CrawlWorker<S, F> {
    db: S,
    fetcher: F,
    wake_publisher: EventWakePublisher,
    ct: CancellationToken,
}

impl<S, F> CrawlWorker<S, F> {
    fn new(db: S, fetcher: F, wake_publisher: EventWakePublisher, ct: CancellationToken) -> Self {
        Self {
            db,
            fetcher,
            wake_publisher,
            ct,
        }
    }
}

impl<S, F> CrawlWorker<S, F>
where
    S: FeedRegistryDb,
    F: FetchFeed + Send + Sync,
    for<'tx> S::Tx<'tx>: BlobStoreTx + CrawlCompletionTx + CrawlJobQueueTx + JournalTx + Send,
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

        let finished_at = Utc::now();
        let recorded = self
            .record_completion(job, outcome, previous_state, finished_at)
            .await?;
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
        finished_at: chrono::DateTime<Utc>,
    ) -> WorkerResult<RecordedEvents> {
        let mut tx = self.db.begin().await?;
        let mut completion_events = RecordedEvents::empty();
        {
            let mut recorder = CrawlCompletionRecorder::new(&mut tx, &mut completion_events);
            recorder
                .record(job, outcome, previous_state, finished_at)
                .await?;
        }
        tx.commit().await?;
        Ok(completion_events)
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
) -> WorkerHandle
where
    S: FeedRegistryDb,
    F: FetchFeed + Clone + Send + Sync + 'static,
    for<'tx> S::Tx<'tx>: BlobStoreTx + CrawlCompletionTx + CrawlJobQueueTx + JournalTx + Send,
{
    CrawlWorkerPool::new(db, fetcher, wake_publisher, poll_interval, config, ct).spawn()
}

#[cfg(test)]
mod tests {
    use super::*;

    mod capacity {
        use super::*;

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
