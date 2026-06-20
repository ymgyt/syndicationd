use chrono::{DateTime, Utc};
use tracing::info;

use crate::{
    crawl::{
        job::EnqueueCrawlJobOutcome,
        schedule::{CrawlScheduleCandidate, CrawlSchedulingEngine},
    },
    db::{CrawlJobQueue, CrawlScheduleStore, FeedRegistryDb},
    event::{
        CrawlJobEnqueuedEvent, Event, EventInput, EventType, InputBatch, Processor, ProcessorId,
        ProcessorResult, Reconciler,
    },
};

const DEFAULT_BATCH_SIZE: usize = 100;

/// Synthetic input used to run a scheduler scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanTick;

impl EventInput for ScanTick {
    const INTERESTS: &'static [EventType] = &[];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        Err(crate::event::ProcessorError::unexpected_input(
            "crawl scheduler scan tick",
            &event,
        ))
    }
}

/// Reconciles crawl scheduling state from durable registry state.
#[derive(Clone)]
pub struct CrawlScheduler {
    batch_size: usize,
}

impl CrawlScheduler {
    pub fn new() -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    pub fn with_batch_size(batch_size: usize) -> Self {
        Self { batch_size }
    }
}

impl Default for CrawlScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for CrawlScheduler {
    type Input = ScanTick;

    fn id(&self) -> ProcessorId {
        ProcessorId::CrawlScheduler
    }
}

impl<S> Reconciler<S> for CrawlScheduler
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlScheduleStore + CrawlJobQueue + Send,
{
    async fn reconcile(
        &mut self,
        tx: &mut S::Tx<'_>,
        now: DateTime<Utc>,
        _batch: InputBatch<Self::Input>,
    ) -> ProcessorResult<Vec<Event>> {
        let mut engine = CrawlSchedulingEngine::new(now);
        let candidates = tx.list_candidates(now, self.batch_size).await?;
        let mut produced = Vec::new();

        for candidate in candidates {
            produced.extend(apply_reconciliation(tx, &mut engine, candidate).await?);
        }

        Ok(produced)
    }
}

async fn apply_reconciliation<Tx>(
    tx: &mut Tx,
    engine: &mut CrawlSchedulingEngine,
    candidate: CrawlScheduleCandidate,
) -> ProcessorResult<Vec<Event>>
where
    Tx: CrawlScheduleStore + CrawlJobQueue + Send,
{
    let reconciliation = engine.reconcile(&candidate);
    let mut events = Vec::new();

    if let Some(schedule) = reconciliation.schedule {
        tx.upsert_schedule(schedule).await?;
    }

    if let Some(job) = reconciliation.job {
        match tx.enqueue_job(job).await? {
            EnqueueCrawlJobOutcome::Enqueued(job) => {
                info!(
                    job_id = %job.job_id,
                    feed_url = job.feed_url.as_str(),
                    trigger = job.trigger.as_str(),
                    queue = job.queue.as_str(),
                    priority = job.priority,
                    run_after = %job.run_after,
                    "crawl job enqueued"
                );
                events.push(CrawlJobEnqueuedEvent::from(job).into());
            }
            EnqueueCrawlJobOutcome::AlreadyActive => {}
        }
    }

    Ok(events)
}
