use std::sync::Arc;

use synd_support::time::{Clock, SystemClock};

use crate::{
    crawl::{
        job::EnqueueCrawlJobOutcome,
        schedule::{CrawlScheduleCandidate, CrawlSchedulingEngine},
    },
    db::{CommitTx, CrawlJobQueueTx, CrawlScheduleTx, FeedRegistryDb},
    event::{
        CrawlTargetActivatedEvent, CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent,
        Event, EventInterests, EventRecorder, EventWorker, JournalAppendTx, JournalTx,
        ReconcileContext, RecordedEvents, RegistryEvent, Trigger, WorkerId, WorkerResult,
    },
};

const DEFAULT_BATCH_SIZE: usize = 100;

/// Reconciles crawl scheduling state from durable registry state.
#[derive(Clone)]
pub struct CrawlScheduler<S> {
    db: S,
    batch_size: usize,
    clock: Arc<dyn Clock>,
}

impl<S> CrawlScheduler<S> {
    pub fn new(db: S) -> Self {
        Self::with_clock(db, Arc::new(SystemClock))
    }

    pub fn with_clock(db: S, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            batch_size: DEFAULT_BATCH_SIZE,
            clock,
        }
    }

    pub fn with_batch_size(db: S, clock: Arc<dyn Clock>, batch_size: usize) -> Self {
        Self {
            db,
            batch_size,
            clock,
        }
    }
}

impl<S> EventWorker for CrawlScheduler<S>
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlScheduleTx + CrawlJobQueueTx + JournalAppendTx + JournalTx + Send,
{
    fn id(&self) -> WorkerId {
        WorkerId::CrawlScheduler
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([
            CrawlTargetActivatedEvent::TYPE,
            CrawlTargetPolicyChangedEvent::TYPE,
            CrawlTargetDeactivatedEvent::TYPE,
        ])
    }

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<RecordedEvents> {
        let now = self.clock.now();
        let mut tx = self.db.begin().await?;
        let mut cx = ReconcileContext::new(&mut tx);
        let mut engine = CrawlSchedulingEngine::new(now);
        let candidates = cx.list_candidates(now, self.batch_size).await?;
        let mut produced = Vec::new();

        for candidate in candidates {
            produced.extend(
                self.apply_reconciliation(&mut cx, &mut engine, candidate)
                    .await?,
            );
        }

        let mut recorded = RecordedEvents::with_capacity(produced.len());
        EventRecorder::new(&mut tx, &mut recorded, self.clock.as_ref())
            .record_all(produced)
            .await?;
        tx.commit().await?;
        Ok(recorded)
    }
}

impl<S> CrawlScheduler<S> {
    async fn apply_reconciliation<Tx>(
        &self,
        cx: &mut ReconcileContext<'_, Tx>,
        engine: &mut CrawlSchedulingEngine,
        candidate: CrawlScheduleCandidate,
    ) -> WorkerResult<Vec<Event>>
    where
        Tx: CrawlScheduleTx + CrawlJobQueueTx + Send,
    {
        let reconciliation = engine.reconcile(&candidate);
        let mut events = Vec::new();

        if let Some(schedule) = reconciliation.schedule {
            cx.upsert_schedule(schedule).await?;
        }

        if let Some(job) = reconciliation.job {
            let mut queue = cx.crawl_job_queue();
            let (outcome, mut produced) = queue.enqueue(job).await?;
            events.append(&mut produced);
            match outcome {
                EnqueueCrawlJobOutcome::Enqueued(_) | EnqueueCrawlJobOutcome::AlreadyActive => {}
            }
        }

        Ok(events)
    }
}
