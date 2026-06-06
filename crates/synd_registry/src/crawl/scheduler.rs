use crate::{
    db::FeedRegistryDb,
    event::{
        CrawlEventKind, EventInterests, ReconcileContext, Reconciler, Trigger, WorkerId,
        WorkerResult,
    },
};

/// Reconciles crawl scheduling state from durable registry state.
#[derive(Debug, Clone)]
pub struct CrawlScheduler;

impl CrawlScheduler {
    pub fn new() -> Self {
        Self
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
        _cx: &mut ReconcileContext<'_, S::Tx<'_>>,
        _trigger: Trigger,
    ) -> WorkerResult<()> {
        Ok(())
    }
}
