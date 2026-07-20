use std::sync::Arc;

use chrono::{DateTime, Utc};
use synd_feed::feed::service::{FeedFetchOutcome, FeedFetchRequest, FeedHttpStatus, FetchFeed};
use synd_support::time::Clock;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::{
    crawl::{
        blob::PutBlobCommand,
        completion::{CrawlCompletion, CrawlCompletionSummary},
        dispatch::{DispatchEntry, DispatchQueueReader},
        job::{CrawlJob, CrawlJobId, CrawlJobQueueLane, CrawlJobTrigger},
        state::{CrawlHealth, CrawlState, UpsertCrawlStateCommand},
    },
    db::{BlobDb, CommitTx, CrawlStateDb, CrawlTargetDb, FeedRegistryDb},
    event::{
        CrawlJobFinishedEvent, EventJournal, EventJournalAppend, EventRecorder, EventWakePublisher,
        RecordedEvents, WorkerHandle, WorkerId, WorkerResult,
    },
};

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

/// Runs crawl dispatch entries handed over through the dispatch queue.
///
/// The pool is queue-driven: it awaits the next dispatched entry, waits for
/// global and lane capacity, then runs the crawl on its own task. The
/// dispatch queue is its only input; it does not consume registry events.
pub(crate) struct CrawlWorkerPool<S, F> {
    db: S,
    fetcher: F,
    wake_publisher: EventWakePublisher,
    dispatch_queue: DispatchQueueReader,
    ct: CancellationToken,
    capacity: CrawlWorkerCapacity,
    clock: Arc<dyn Clock>,
}

impl<S, F> CrawlWorkerPool<S, F> {
    pub(crate) fn new(
        db: S,
        fetcher: F,
        wake_publisher: EventWakePublisher,
        dispatch_queue: DispatchQueueReader,
        config: CrawlWorkerPoolConfig,
        ct: CancellationToken,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            db,
            fetcher,
            wake_publisher,
            dispatch_queue,
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
        BlobDb + CrawlStateDb + CrawlTargetDb + EventJournalAppend + EventJournal + Send,
{
    pub(crate) fn spawn(self) -> WorkerHandle {
        WorkerHandle::new(WorkerId::CrawlWorkerPool, tokio::spawn(self.run()))
    }

    async fn run(mut self) {
        debug!(
            worker = WorkerId::CrawlWorkerPool.as_str(),
            "crawl worker pool started"
        );

        loop {
            let entry = tokio::select! {
                () = self.ct.cancelled() => break,
                entry = self.dispatch_queue.recv() => match entry {
                    Some(entry) => entry,
                    None => break,
                },
            };
            let lane = entry.trigger.queue_lane();
            let slot = tokio::select! {
                () = self.ct.cancelled() => break,
                slot = self.capacity.reserve(lane) => slot,
            };
            self.start_worker(entry, slot);
        }

        debug!(
            worker = WorkerId::CrawlWorkerPool.as_str(),
            "crawl worker pool stopped"
        );
    }

    fn start_worker(&self, entry: DispatchEntry, slot: CrawlWorkerSlot) {
        let db = self.db.clone();
        let fetcher = self.fetcher.clone();
        let wake_publisher = self.wake_publisher.clone();
        let worker_ct = self.ct.child_token();
        let clock = Arc::clone(&self.clock);
        let (job, inflight) = entry.into_crawl_job();
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
            // The inflight claim releases only after the completion commit,
            // so the dispatcher never double-dispatches a running feed.
            drop(inflight);
            drop(slot);
        });
    }
}

/// Runs one dispatched crawl through fetch, classification, and completion
/// recording.
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
        BlobDb + CrawlStateDb + CrawlTargetDb + EventJournalAppend + EventJournal + Send,
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
        let started_at = job.started_at;
        let previous_state = self.load_previous_state(&job).await?;
        let request = job.fetch_request(previous_state.as_ref());

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
        let (summary, recorded) = self
            .record_completion(job, outcome, previous_state, finished_at)
            .await?;
        summary.log_completed(
            &job_id,
            feed_url.as_str(),
            lane,
            trigger,
            (finished_at - started_at).num_milliseconds(),
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

    /// Records what the finished crawl leaves behind in one transaction:
    /// the body blob, the crawl-state summary, the served manual request,
    /// and the `CrawlJobFinished` fact.
    async fn record_completion(
        &self,
        job: CrawlJob,
        outcome: FeedFetchOutcome,
        previous_state: Option<CrawlState>,
        finished_at: DateTime<Utc>,
    ) -> WorkerResult<(CrawlCompletionSummary, RecordedEvents)> {
        let previous_conditional = previous_state
            .as_ref()
            .map(|state| state.conditional.clone())
            .unwrap_or_default();
        let completion =
            CrawlCompletion::classify(outcome, job.started_at, finished_at, &previous_conditional);
        let health = CrawlHealth::for_last_result(&completion.last, previous_state.as_ref());

        let mut tx = self.db.begin().await?;
        let body_blob = match completion.body {
            Some(bytes) => Some(tx.put_blob(PutBlobCommand::new(bytes, finished_at)).await?),
            None => None,
        };
        tx.upsert_crawl_state(UpsertCrawlStateCommand::new(
            job.feed_url.clone(),
            completion.last,
            health,
            completion.conditional,
        ))
        .await?;
        tx.clear_manual_request(&job.feed_url, job.started_at)
            .await?;

        let mut recorded_events = RecordedEvents::with_capacity(1);
        EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref())
            .record(CrawlJobFinishedEvent::new(
                job.job_id,
                job.feed_url,
                job.started_at,
                body_blob,
            ))
            .await?;
        tx.commit().await?;
        Ok((completion.summary, recorded_events))
    }
}

impl CrawlCompletionSummary {
    fn log_completed(
        &self,
        job_id: &CrawlJobId,
        feed_url: &str,
        lane: CrawlJobQueueLane,
        trigger: CrawlJobTrigger,
        duration_ms: i64,
    ) {
        // `Option` fields are recorded only when present.
        info!(
            job_id = %job_id,
            feed_url,
            queue = lane.as_str(),
            trigger = trigger.as_str(),
            outcome = self.outcome.as_str(),
            http_status = self.http_status.map(FeedHttpStatus::as_u16),
            error_kind = self.error_kind,
            duration_ms,
            "crawl job completed"
        );
    }
}

impl DispatchEntry {
    fn into_crawl_job(self) -> (CrawlJob, super::dispatch::InflightGuard) {
        (
            CrawlJob::new(
                CrawlJobId::generate(),
                self.feed_url,
                self.trigger,
                self.dispatched_at,
            ),
            self.inflight,
        )
    }
}

impl CrawlJob {
    /// Builds the conditional fetch request for this job from the previous
    /// crawl state.
    fn fetch_request(&self, previous_state: Option<&CrawlState>) -> FeedFetchRequest {
        let conditional = previous_state
            .map(|state| state.conditional.clone())
            .unwrap_or_default();
        FeedFetchRequest::new(self.feed_url.clone()).with_conditional(conditional)
    }
}

/// Runtime permits controlling global and lane-local crawl concurrency.
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

    /// Waits until both a global and a lane slot are available.
    ///
    /// The global permit is held while waiting for the lane permit, so a
    /// saturated lane blocks subsequent dispatches (head-of-line), matching
    /// the single-consumer dispatch queue semantics.
    async fn reserve(&self, lane: CrawlJobQueueLane) -> CrawlWorkerSlot {
        let global_permit = Arc::clone(&self.global)
            .acquire_owned()
            .await
            .expect("crawl worker global semaphore is never closed");
        let lane_permit = self
            .lane_capacity(lane)
            .acquire_owned()
            .await
            .expect("crawl worker lane semaphore is never closed");

        CrawlWorkerSlot {
            lane,
            _global_permit: global_permit,
            _lane_permit: lane_permit,
        }
    }

    #[cfg(test)]
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

/// Acquired capacity permits for one running crawl job.
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

#[cfg(test)]
mod tests {
    use super::{
        CrawlWorkerCapacity, CrawlWorkerFetchConfig, CrawlWorkerPoolConfig, CrawlWorkerQueueConfig,
    };
    use crate::crawl::job::CrawlJobQueueLane;

    mod capacity {
        use super::{CrawlJobQueueLane, CrawlWorkerCapacity, config};

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
