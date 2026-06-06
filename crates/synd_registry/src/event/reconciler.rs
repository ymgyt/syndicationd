use std::{future::Future, time::Duration};

use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    db::{CommitTx, FeedRegistryDb},
    event::{
        EventInterests, EventWake, EventWakePublisher, EventWakeRecvError, ReconcileContext,
        RecordedEvents, Trigger, WorkerHandle, WorkerId, WorkerResult,
    },
};

/// Reconciles durable registry state without using an event journal cursor.
pub(crate) trait Reconciler<S>: Send + 'static
where
    S: FeedRegistryDb,
{
    fn id(&self) -> WorkerId;

    fn interests(&self) -> EventInterests;

    fn reconcile(
        &mut self,
        cx: &mut ReconcileContext<'_, S::Tx<'_>>,
        trigger: Trigger,
    ) -> impl Future<Output = WorkerResult<()>> + Send;
}

/// Runs one reconciler as an asynchronous registry worker.
pub(crate) struct ReconcilerWorker<S, R> {
    db: S,
    reconciler: R,
    wake: EventWake,
    poll_interval: Duration,
}

impl<S, R> ReconcilerWorker<S, R> {
    pub fn new(db: S, reconciler: R, wake: EventWake, poll_interval: Duration) -> Self {
        Self {
            db,
            reconciler,
            wake,
            poll_interval,
        }
    }
}

impl<S, R> ReconcilerWorker<S, R>
where
    S: FeedRegistryDb,
    R: Reconciler<S>,
{
    pub fn spawn(self, ct: CancellationToken) -> WorkerHandle {
        let worker = self.reconciler.id();
        WorkerHandle::new(worker, tokio::spawn(self.run(ct)))
    }

    async fn run(mut self, ct: CancellationToken) {
        let worker = self.reconciler.id();
        debug!(
            worker = worker.as_str(),
            "registry reconciler worker started"
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
                        Ok(recorded) if self.reconciler.interests().matches_any(recorded.kinds()) => {
                            self.process(Trigger::Wake).await;
                        }
                        Ok(_) => {}
                        Err(EventWakeRecvError::Lagged(skipped)) => {
                            warn!(
                                worker = worker.as_str(),
                                skipped,
                                "registry reconciler worker wake lagged"
                            );
                            self.process(Trigger::WakeLagged).await;
                        }
                        Err(EventWakeRecvError::Closed) => {
                            warn!(
                                worker = worker.as_str(),
                                "registry reconciler worker wake channel closed"
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
            worker = worker.as_str(),
            "registry reconciler worker stopped"
        );
    }

    #[tracing::instrument(
        name = "registry.reconciler.worker.process",
        skip_all,
        fields(
            worker = self.reconciler.id().as_str(),
            trigger = trigger.as_str()
        )
    )]
    async fn process(&mut self, trigger: Trigger) {
        let worker = self.reconciler.id();
        match self.process_transaction(trigger).await {
            Ok(recorded) => {
                let recorded_count = recorded.len();
                let receivers = self.wake.publish(recorded);
                debug!(
                    worker = worker.as_str(),
                    trigger = trigger.as_str(),
                    recorded_count,
                    receivers,
                    "registry reconciler worker processed"
                );
            }
            Err(err) => {
                error!(
                    worker = worker.as_str(),
                    trigger = trigger.as_str(),
                    error = %err,
                    "registry reconciler worker failed"
                );
            }
        }
    }

    async fn process_transaction(&mut self, trigger: Trigger) -> WorkerResult<RecordedEvents> {
        let mut tx = self.db.begin().await?;
        let mut cx = ReconcileContext::new(&mut tx);
        self.reconciler.reconcile(&mut cx, trigger).await?;
        let recorded = cx.into_recorded();
        tx.commit().await?;
        Ok(recorded)
    }
}

pub(crate) fn spawn_reconciler_worker<S, R>(
    db: S,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    reconciler: R,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    R: Reconciler<S>,
{
    ReconcilerWorker::new(
        db,
        reconciler,
        EventWake::new(wake_publisher),
        poll_interval,
    )
    .spawn(ct)
}
