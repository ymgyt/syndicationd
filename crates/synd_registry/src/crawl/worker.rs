use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    crawl::{
        job::{ClaimCrawlJobCommand, ClaimCrawlJobOutcome, CrawlJob, CrawlJobQueueLane},
        queue::CrawlJobQueue,
    },
    db::{CommitTx, CrawlJobQueueTx, FeedRegistryDb},
    event::{
        CrawlEventKind, EventInterests, EventWake, EventWakePublisher, EventWakeRecvError,
        RecordedEvents, Trigger, WorkerHandle, WorkerId, WorkerResult,
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
        }
    }
}

/// Queue-local crawl worker pool configuration.
#[derive(Debug, Clone, Copy)]
pub struct CrawlWorkerQueueConfig {
    pub max_running_jobs: usize,
}

/// Claims and runs durable crawl jobs.
pub(crate) struct CrawlWorkerPool<S> {
    db: S,
    wake: EventWake,
    poll_interval: Duration,
    ct: CancellationToken,
    capacity: CrawlWorkerCapacity,
}

impl<S> CrawlWorkerPool<S> {
    pub fn new(
        db: S,
        wake: EventWake,
        poll_interval: Duration,
        config: CrawlWorkerPoolConfig,
        ct: CancellationToken,
    ) -> Self {
        Self {
            db,
            wake,
            poll_interval,
            ct,
            capacity: CrawlWorkerCapacity::new(config),
        }
    }
}

impl<S> CrawlWorkerPool<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlJobQueueTx,
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
        let worker_ct = self.ct.child_token();
        tokio::spawn(async move {
            debug!(
                worker = WorkerId::CrawlWorkerPool.as_str(),
                job_id = %job.job_id,
                queue = slot.lane().as_str(),
                "crawl worker started job"
            );
            worker_ct.cancelled().await;
            drop(slot);
        });
    }
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

pub(crate) fn spawn_crawl_worker_pool<S>(
    db: S,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    config: CrawlWorkerPoolConfig,
    ct: CancellationToken,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlJobQueueTx,
{
    CrawlWorkerPool::new(
        db,
        EventWake::new(wake_publisher),
        poll_interval,
        config,
        ct,
    )
    .spawn()
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
        }
    }
}
