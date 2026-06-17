use std::{fmt, future::Future, sync::Arc, time::Duration};

use synd_support::time::Clock;
use thiserror::Error;
use tokio::{sync::broadcast, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    db::{CommitTx, FeedRegistryDb},
    error::RegistryDbError,
    event::{
        CursorRole, EventInput, EventInterests, EventJournal, EventJournalAppend, EventReadBatch,
        EventRecorder, InputBatch, Processor, ProcessorError, ProcessorId, Reconciler,
        RecordedEvents, Sink, skip_permanent_error,
    },
};

/// Result type returned by registry event workers.
pub type WorkerResult<T> = Result<T, WorkerError>;

/// Error returned while an event worker processes the journal.
#[derive(Debug, Error)]
pub enum WorkerError {
    #[error(transparent)]
    RegistryDb(#[from] RegistryDbError),
    #[error(transparent)]
    Processor(#[from] ProcessorError),
}

/// Source that caused an event worker processing attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Startup,
    Wake,
    WakeLagged,
    Poll,
}

impl Trigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Wake => "wake",
            Self::WakeLagged => "wake_lagged",
            Self::Poll => "poll",
        }
    }
}

/// Stable identity for a background worker task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkerId {
    Processor(ProcessorId),
    CrawlScheduler,
    CrawlWorkerPool,
}

impl WorkerId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Processor(processor) => processor.as_str(),
            Self::CrawlScheduler => "CrawlScheduler",
            Self::CrawlWorkerPool => "CrawlWorkerPool",
        }
    }
}

impl From<ProcessorId> for WorkerId {
    fn from(processor: ProcessorId) -> Self {
        Self::Processor(processor)
    }
}

/// Error returned while receiving a journal wake notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventWakeRecvError {
    Closed,
    Lagged(u64),
}

/// Publishes journal wake notifications to event workers.
#[derive(Clone)]
pub struct EventWakePublisher {
    sender: broadcast::Sender<RecordedEvents>,
}

impl fmt::Debug for EventWakePublisher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventWakePublisher").finish_non_exhaustive()
    }
}

/// Receives journal wake notifications for one event worker.
pub struct EventWakeSubscriber {
    receiver: broadcast::Receiver<RecordedEvents>,
}

/// Worker-local connection to the registry event wake channel.
pub struct EventWake {
    publisher: EventWakePublisher,
    subscriber: EventWakeSubscriber,
}

/// Owns the task running one event processor.
#[derive(Debug)]
pub struct WorkerHandle {
    id: WorkerId,
    task: JoinHandle<()>,
}

/// Owns the set of registry event worker tasks.
#[derive(Debug)]
pub struct WorkerSet {
    handles: Vec<WorkerHandle>,
}

impl EventWakePublisher {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> EventWakeSubscriber {
        EventWakeSubscriber {
            receiver: self.sender.subscribe(),
        }
    }

    pub fn publish(&self, recorded: RecordedEvents) -> usize {
        if recorded.is_empty() {
            return 0;
        }
        self.sender.send(recorded).unwrap_or_default()
    }
}

impl EventWakeSubscriber {
    pub async fn recv(&mut self) -> Result<RecordedEvents, EventWakeRecvError> {
        self.receiver.recv().await.map_err(|err| match err {
            broadcast::error::RecvError::Closed => EventWakeRecvError::Closed,
            broadcast::error::RecvError::Lagged(skipped) => EventWakeRecvError::Lagged(skipped),
        })
    }
}

impl EventWake {
    pub fn new(publisher: EventWakePublisher) -> Self {
        let subscriber = publisher.subscribe();
        Self {
            publisher,
            subscriber,
        }
    }

    pub async fn recv(&mut self) -> Result<RecordedEvents, EventWakeRecvError> {
        self.subscriber.recv().await
    }

    pub fn publish(&self, recorded: RecordedEvents) -> usize {
        self.publisher.publish(recorded)
    }
}

impl WorkerHandle {
    pub fn new(id: impl Into<WorkerId>, task: JoinHandle<()>) -> Self {
        Self {
            id: id.into(),
            task,
        }
    }

    pub fn id(&self) -> WorkerId {
        self.id
    }

    pub fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    pub fn abort(&self) {
        self.task.abort();
    }

    pub async fn join(self) -> Result<(), tokio::task::JoinError> {
        self.task.await
    }
}

impl WorkerSet {
    pub fn new(handles: Vec<WorkerHandle>) -> Self {
        Self { handles }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn handles(&self) -> &[WorkerHandle] {
        &self.handles
    }

    pub fn len(&self) -> usize {
        self.handles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.handles.is_empty()
    }

    pub fn abort(&self) {
        for handle in &self.handles {
            handle.abort();
        }
    }

    pub async fn join(mut self) -> Vec<(WorkerId, Result<(), tokio::task::JoinError>)> {
        let handles = std::mem::take(&mut self.handles);
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let id = handle.id();
            results.push((id, handle.join().await));
        }
        results
    }
}

impl Drop for WorkerSet {
    fn drop(&mut self) {
        self.abort();
    }
}

/// A wake-driven registry worker with one unit of reaction logic.
pub(crate) trait EventWorker: Send + 'static {
    fn id(&self) -> WorkerId;

    fn interests(&self) -> EventInterests;

    fn react(
        &mut self,
        trigger: Trigger,
    ) -> impl Future<Output = WorkerResult<RecordedEvents>> + Send;
}

pub(crate) fn spawn_event_loop<W>(
    worker: W,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
) -> WorkerHandle
where
    W: EventWorker,
{
    let id = worker.id();
    WorkerHandle::new(
        id,
        tokio::spawn(run_event_loop(
            worker,
            EventWake::new(wake_publisher),
            poll_interval,
            ct,
        )),
    )
}

async fn run_event_loop<W>(
    mut worker: W,
    mut wake: EventWake,
    poll_interval: Duration,
    ct: CancellationToken,
) where
    W: EventWorker,
{
    let id = worker.id();
    debug!(worker = id.as_str(), "registry event worker started");
    process_event_worker(&mut worker, &wake, Trigger::Startup).await;

    let mut interval = tokio::time::interval(poll_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval.tick().await;

    loop {
        tokio::select! {
            () = ct.cancelled() => break,
            received = wake.recv() => {
                match received {
                    Ok(recorded) if worker.interests().matches_any(recorded.types()) => {
                        process_event_worker(&mut worker, &wake, Trigger::Wake).await;
                    }
                    Ok(_) => {}
                    Err(EventWakeRecvError::Lagged(skipped)) => {
                        warn!(
                            worker = id.as_str(),
                            skipped,
                            "registry event worker wake lagged"
                        );
                        process_event_worker(&mut worker, &wake, Trigger::WakeLagged).await;
                    }
                    Err(EventWakeRecvError::Closed) => {
                        warn!(
                            worker = id.as_str(),
                            "registry event worker wake channel closed"
                        );
                        break;
                    }
                }
            }
            _ = interval.tick() => {
                process_event_worker(&mut worker, &wake, Trigger::Poll).await;
            }
        }
    }

    debug!(worker = id.as_str(), "registry event worker stopped");
}

#[tracing::instrument(
    name = "registry.event.worker.react",
    skip_all,
    fields(worker = worker.id().as_str(), trigger = trigger.as_str())
)]
async fn process_event_worker<W>(worker: &mut W, wake: &EventWake, trigger: Trigger)
where
    W: EventWorker,
{
    let id = worker.id();
    match worker.react(trigger).await {
        Ok(recorded) => {
            let recorded_count = recorded.len();
            let receivers = wake.publish(recorded);
            debug!(
                worker = id.as_str(),
                trigger = trigger.as_str(),
                recorded_count,
                receivers,
                "registry event worker reacted"
            );
        }
        Err(err) => {
            error!(
                worker = id.as_str(),
                trigger = trigger.as_str(),
                error = %err,
                "registry event worker failed"
            );
        }
    }
}

/// Adapts a transactional event consumer to the shared wake-driven loop.
pub(crate) struct CursorAdapter<S, P> {
    db: S,
    processor: P,
    clock: Arc<dyn Clock>,
}

impl<S, P> CursorAdapter<S, P> {
    pub fn new(db: S, processor: P, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            processor,
            clock,
        }
    }
}

impl<S, P> EventWorker for CursorAdapter<S, P>
where
    S: FeedRegistryDb,
    P: CursorRole<S>,
    for<'tx> S::Tx<'tx>: EventJournalAppend,
{
    fn id(&self) -> WorkerId {
        self.processor.id().into()
    }

    fn interests(&self) -> EventInterests {
        self.processor.interests()
    }

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<RecordedEvents> {
        let processor_id = self.processor.id();
        let mut tx = self.db.begin().await?;
        let cursor = tx.load_cursor(processor_id).await?;
        let batch = tx.read_after(&cursor, self.processor.interests()).await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();

        let inputs = collect_inputs::<P>(processor_id, batch)?;
        let produced = self
            .processor
            .process_cursor_batch(&mut tx, self.clock.now(), InputBatch::new(inputs))
            .await?;
        let mut recorded_events = RecordedEvents::with_capacity(produced.len());
        {
            let mut event_recorder =
                EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref());
            event_recorder.record_all(produced).await?;
        }

        tx.advance_cursor(&scanned_cursor).await?;
        tx.commit().await?;

        debug!(
            worker = self.id().as_str(),
            event_count,
            cursor_position = ?scanned_cursor.position(),
            "registry cursor worker advanced"
        );

        Ok(recorded_events)
    }
}

/// Adapts an idempotent scan reconciler to the shared wake-driven loop.
pub(crate) struct ScanAdapter<S, P> {
    db: S,
    processor: P,
    clock: Arc<dyn Clock>,
}

impl<S, P> ScanAdapter<S, P> {
    pub fn new(db: S, processor: P, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            processor,
            clock,
        }
    }
}

impl<S, P> EventWorker for ScanAdapter<S, P>
where
    S: FeedRegistryDb,
    P: Reconciler<S>,
    for<'tx> S::Tx<'tx>: EventJournalAppend,
{
    fn id(&self) -> WorkerId {
        self.processor.id().into()
    }

    fn interests(&self) -> EventInterests {
        EventInterests::empty()
    }

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<RecordedEvents> {
        let mut tx = self.db.begin().await?;
        let produced = self
            .processor
            .reconcile(&mut tx, self.clock.now(), InputBatch::new(Vec::new()))
            .await?;
        let mut recorded_events = RecordedEvents::with_capacity(produced.len());
        {
            let mut event_recorder =
                EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref());
            event_recorder.record_all(produced).await?;
        }
        tx.commit().await?;

        debug!(
            worker = self.id().as_str(),
            recorded_count = recorded_events.len(),
            "registry scan worker reconciled"
        );

        Ok(recorded_events)
    }
}

/// Adapts a post-commit sink to the shared wake-driven loop.
pub(crate) struct PostCommitAdapter<S, P> {
    db: S,
    processor: P,
}

impl<S, P> PostCommitAdapter<S, P> {
    pub fn new(db: S, processor: P) -> Self {
        Self { db, processor }
    }
}

impl<S, P> EventWorker for PostCommitAdapter<S, P>
where
    S: FeedRegistryDb,
    P: Sink,
{
    fn id(&self) -> WorkerId {
        self.processor.id().into()
    }

    fn interests(&self) -> EventInterests {
        self.processor.interests()
    }

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<RecordedEvents> {
        let processor_id = self.processor.id();
        let mut tx = self.db.begin().await?;
        let cursor = tx.load_cursor(processor_id).await?;
        let batch = tx.read_after(&cursor, self.processor.interests()).await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();
        let inputs = collect_inputs::<P>(processor_id, batch)?;

        tx.advance_cursor(&scanned_cursor).await?;
        tx.commit().await?;

        for input in inputs {
            self.processor.deliver(input).await;
        }

        debug!(
            worker = self.id().as_str(),
            event_count,
            cursor_position = ?scanned_cursor.position(),
            "registry post-commit worker advanced"
        );

        Ok(RecordedEvents::empty())
    }
}

fn collect_inputs<P>(processor: ProcessorId, batch: EventReadBatch) -> WorkerResult<Vec<P::Input>>
where
    P: Processor,
{
    let mut inputs = Vec::with_capacity(batch.events().len());
    for journaled in batch.into_events() {
        let occurred_at = journaled.occurred_at();
        match P::Input::from_event(journaled.into_event(), occurred_at) {
            Ok(input) => inputs.push(input),
            Err(err) => skip_permanent_error(processor, err, "event input")?,
        }
    }
    Ok(inputs)
}
