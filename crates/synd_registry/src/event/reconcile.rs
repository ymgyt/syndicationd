use std::{future::Future, sync::Arc};

use chrono::{DateTime, Utc};
use synd_support::time::Clock;

use crate::{
    db::FeedRegistryDb,
    event::{EventInterests, EventWorker, Reaction, Trigger, WorkerId, WorkerResult},
};

/// A level-driven convergence loop over durable state.
///
/// Unlike a [`crate::event::Projector`], a reconciler does not consume the
/// event journal: every pass re-reads current durable state and converges it
/// toward the desired state, so a missed wake is recovered by the next poll
/// tick. Journal events act only as hints that the observed level may have
/// changed; they are never inputs and no cursor is kept.
///
/// Implementations must be idempotent: reconciling an already-converged state
/// is a no-op.
pub(crate) trait Reconciler<S>: Send + 'static
where
    S: FeedRegistryDb,
{
    fn id(&self) -> WorkerId;

    /// Event types hinting that the observed level may have changed.
    fn wake_hints(&self) -> EventInterests;

    /// Observes durable state and converges it toward the desired state.
    ///
    /// The reconciler owns its transactions, so side effects that must happen
    /// strictly after commit (e.g. handing work to an in-process queue) can be
    /// sequenced correctly.
    fn reconcile(
        &mut self,
        db: &S,
        now: DateTime<Utc>,
    ) -> impl Future<Output = WorkerResult<Reaction>> + Send;
}

/// Runs one reconciler on the shared wake-driven loop.
pub(crate) struct ReconcilerWorker<S, R> {
    db: S,
    reconciler: R,
    clock: Arc<dyn Clock>,
}

impl<S, R> ReconcilerWorker<S, R> {
    pub(crate) fn new(db: S, reconciler: R, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            reconciler,
            clock,
        }
    }
}

impl<S, R> EventWorker for ReconcilerWorker<S, R>
where
    S: FeedRegistryDb,
    R: Reconciler<S>,
{
    fn id(&self) -> WorkerId {
        self.reconciler.id()
    }

    fn interests(&self) -> EventInterests {
        self.reconciler.wake_hints()
    }

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<Reaction> {
        self.reconciler.reconcile(&self.db, self.clock.now()).await
    }
}
