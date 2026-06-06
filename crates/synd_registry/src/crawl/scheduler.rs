use chrono::Utc;

use crate::{
    crawl::{
        job::EnqueueCrawlJobOutcome,
        schedule::{CrawlScheduleCandidate, CrawlSchedulingEngine},
    },
    db::{CrawlJobQueueTx, CrawlScheduleTx, FeedRegistryDb},
    event::{
        CrawlEventKind, EventInterests, ReconcileContext, Reconciler, Trigger, WorkerId,
        WorkerResult,
    },
};

const DEFAULT_BATCH_SIZE: usize = 100;

/// Reconciles crawl scheduling state from durable registry state.
#[derive(Debug, Clone)]
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

impl<S> Reconciler<S> for CrawlScheduler
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: CrawlScheduleTx + CrawlJobQueueTx,
{
    fn id(&self) -> WorkerId {
        WorkerId::CrawlScheduler
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([
            CrawlEventKind::TargetActivated.into(),
            CrawlEventKind::TargetPolicyChanged.into(),
            CrawlEventKind::TargetDeactivated.into(),
        ])
    }

    async fn reconcile(
        &mut self,
        cx: &mut ReconcileContext<'_, S::Tx<'_>>,
        _trigger: Trigger,
    ) -> WorkerResult<()> {
        let now = Utc::now();
        let mut engine = CrawlSchedulingEngine::new(now);
        let candidates = cx.list_candidates(now, self.batch_size).await?;

        for candidate in candidates {
            self.apply_reconciliation(cx, &mut engine, candidate)
                .await?;
        }

        Ok(())
    }
}

impl CrawlScheduler {
    async fn apply_reconciliation<Tx>(
        &self,
        cx: &mut ReconcileContext<'_, Tx>,
        engine: &mut CrawlSchedulingEngine,
        candidate: CrawlScheduleCandidate,
    ) -> WorkerResult<()>
    where
        Tx: CrawlScheduleTx + CrawlJobQueueTx + crate::event::JournalTx + Send,
    {
        let reconciliation = engine.reconcile(&candidate);

        if let Some(schedule) = reconciliation.schedule {
            cx.upsert_schedule(schedule).await?;
        }

        if let Some(job) = reconciliation.job {
            let mut queue = cx.crawl_job_queue();
            match queue.enqueue(job).await? {
                EnqueueCrawlJobOutcome::Enqueued(_) | EnqueueCrawlJobOutcome::AlreadyActive => {}
            }
        }

        Ok(())
    }
}
