use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    db::{CommitTx, CrawlJobQueueTx, FeedRegistryDb},
    event::{
        CrawlEventKind, EventInterests, EventWake, EventWakePublisher, EventWakeRecvError, Trigger,
        WorkerHandle, WorkerId, WorkerResult,
    },
};

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
                weight: 4,
                max_running_jobs: 2,
            },
            default_queue: CrawlWorkerQueueConfig {
                weight: 3,
                max_running_jobs: 4,
            },
            retry_queue: CrawlWorkerQueueConfig {
                weight: 1,
                max_running_jobs: 1,
            },
        }
    }
}

/// Queue-local crawl worker pool configuration.
#[derive(Debug, Clone, Copy)]
pub struct CrawlWorkerQueueConfig {
    pub weight: usize,
    pub max_running_jobs: usize,
}

/// Claims and runs durable crawl jobs.
pub(crate) struct CrawlWorkerPool<S> {
    db: S,
    wake: EventWake,
    poll_interval: Duration,
    config: CrawlWorkerPoolConfig,
}

impl<S> CrawlWorkerPool<S> {
    pub fn new(
        db: S,
        wake: EventWake,
        poll_interval: Duration,
        config: CrawlWorkerPoolConfig,
    ) -> Self {
        Self {
            db,
            wake,
            poll_interval,
            config,
        }
    }
}

impl<S> CrawlWorkerPool<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlJobQueueTx,
{
    pub fn spawn(self, ct: CancellationToken) -> WorkerHandle {
        WorkerHandle::new(WorkerId::CrawlWorkerPool, tokio::spawn(self.run(ct)))
    }

    async fn run(mut self, ct: CancellationToken) {
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
                () = ct.cancelled() => break,
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
        match self.process_transaction().await {
            Ok(report) => {
                debug!(
                    worker = WorkerId::CrawlWorkerPool.as_str(),
                    trigger = trigger.as_str(),
                    pending_count = report.pending_count,
                    running_count = report.running_count,
                    available_capacity = report.available_capacity,
                    "crawl worker pool observed queue"
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

    async fn process_transaction(&mut self) -> WorkerResult<CrawlWorkerPoolReport> {
        let mut tx = self.db.begin().await?;
        let snapshot = tx.queue_snapshot().await?;
        tx.commit().await?;

        let running_count = usize::try_from(snapshot.running_count).unwrap_or(usize::MAX);
        let available_capacity = self.config.max_running_jobs.saturating_sub(running_count);

        Ok(CrawlWorkerPoolReport {
            pending_count: snapshot.pending_count,
            running_count: snapshot.running_count,
            available_capacity,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CrawlWorkerPoolReport {
    pending_count: u64,
    running_count: u64,
    available_capacity: usize,
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
    CrawlWorkerPool::new(db, EventWake::new(wake_publisher), poll_interval, config).spawn(ct)
}
