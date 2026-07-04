use std::{fmt, future::Future, sync::Arc, time::Duration};

use chrono::{DateTime, Utc};
use synd_support::time::Clock;
use thiserror::Error;
use tokio::{sync::broadcast, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    db::{CommitTx, FeedRegistryDb},
    error::RegistryDbError,
    event::{
        EventCursor, EventInput, EventInterests, EventJournal, EventJournalAppend, EventRecorder,
        InputBatch, ProcessorError, ProcessorId, Projector, Reaction, RecordedEvents, Sink,
        WakeRequest,
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
    ScheduledWake,
    Poll,
}

impl Trigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Startup => "startup",
            Self::Wake => "wake",
            Self::WakeLagged => "wake_lagged",
            Self::ScheduledWake => "scheduled_wake",
            Self::Poll => "poll",
        }
    }
}

/// Stable identity for a background worker task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkerId {
    Processor(ProcessorId),
    CrawlDispatcher,
    CrawlWorkerPool,
}

impl WorkerId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Processor(processor) => processor.as_str(),
            Self::CrawlDispatcher => "CrawlDispatcher",
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

/// Receives journal wake notifications for one event worker.
pub struct EventWakeSubscriber {
    receiver: broadcast::Receiver<RecordedEvents>,
}

impl EventWakeSubscriber {
    pub async fn recv(&mut self) -> Result<RecordedEvents, EventWakeRecvError> {
        self.receiver.recv().await.map_err(|err| match err {
            broadcast::error::RecvError::Closed => EventWakeRecvError::Closed,
            broadcast::error::RecvError::Lagged(skipped) => EventWakeRecvError::Lagged(skipped),
        })
    }
}

/// Worker-local connection to the registry event wake channel.
pub struct EventWake {
    publisher: EventWakePublisher,
    subscriber: EventWakeSubscriber,
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

/// Owns the task running one event processor.
#[derive(Debug)]
pub struct WorkerHandle {
    id: WorkerId,
    task: JoinHandle<()>,
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

/// Owns the set of registry event worker tasks.
#[derive(Debug)]
pub struct WorkerSet {
    handles: Vec<WorkerHandle>,
}

impl WorkerSet {
    pub fn new(handles: Vec<WorkerHandle>) -> Self {
        Self { handles }
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

    fn react(&mut self, trigger: Trigger) -> impl Future<Output = WorkerResult<Reaction>> + Send;
}

impl WakeRequest {
    /// Resolves at the requested instant; pends forever when no wake was
    /// requested.
    pub(crate) async fn wait(self) {
        match self {
            Self::None => std::future::pending().await,
            Self::At(wake_at) => tokio::time::sleep_until(Self::instant(wake_at)).await,
        }
    }

    fn instant(wake_at: DateTime<Utc>) -> tokio::time::Instant {
        let now = Utc::now();
        if wake_at <= now {
            return tokio::time::Instant::now();
        }
        let delay = wake_at
            .signed_duration_since(now)
            .to_std()
            .unwrap_or(Duration::ZERO);
        tokio::time::Instant::now() + delay
    }
}

/// Drives one event worker on its own task, reacting to journal wakes that
/// match the worker's interests, self-requested timer wakes, and a poll
/// fallback.
pub(crate) struct EventLoop<W> {
    worker: W,
    wake: EventWake,
    poll_interval: Duration,
    ct: CancellationToken,
}

impl<W> EventLoop<W>
where
    W: EventWorker,
{
    pub(crate) fn new(
        worker: W,
        wake_publisher: EventWakePublisher,
        poll_interval: Duration,
        ct: CancellationToken,
    ) -> Self {
        Self {
            worker,
            wake: EventWake::new(wake_publisher),
            poll_interval,
            ct,
        }
    }

    pub(crate) fn spawn(self) -> WorkerHandle {
        WorkerHandle::new(self.worker.id(), tokio::spawn(self.run()))
    }

    async fn run(mut self) {
        let id = self.worker.id();
        debug!(worker = id.as_str(), "registry event worker started");
        let mut requested_wake = self.react(Trigger::Startup).await;

        let mut poll = tokio::time::interval(self.poll_interval);
        poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        poll.tick().await;

        loop {
            let scheduled_wake = requested_wake.wait();
            tokio::pin!(scheduled_wake);
            tokio::select! {
                () = self.ct.cancelled() => break,
                () = &mut scheduled_wake => {
                    requested_wake = self.react(Trigger::ScheduledWake).await;
                }
                received = self.wake.recv() => {
                    match received {
                        Ok(recorded) if self.worker.interests().matches_any(recorded.types()) => {
                            requested_wake = self.react(Trigger::Wake).await;
                        }
                        Ok(_) => {}
                        Err(EventWakeRecvError::Lagged(skipped)) => {
                            warn!(
                                worker = id.as_str(),
                                skipped,
                                "registry event worker wake lagged"
                            );
                            requested_wake = self.react(Trigger::WakeLagged).await;
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
                _ = poll.tick() => {
                    requested_wake = self.react(Trigger::Poll).await;
                }
            }
        }

        debug!(worker = id.as_str(), "registry event worker stopped");
    }

    /// Runs one worker reaction and publishes its recorded events as wakes.
    #[tracing::instrument(
        name = "registry.event.worker.react",
        skip_all,
        fields(worker = self.worker.id().as_str(), trigger = trigger.as_str())
    )]
    async fn react(&mut self, trigger: Trigger) -> WakeRequest {
        let id = self.worker.id();
        match self.worker.react(trigger).await {
            Ok(reaction) => {
                let (recorded, wake_request) = reaction.into_parts();
                let recorded_count = recorded.len();
                let receivers = self.wake.publish(recorded);
                debug!(
                    worker = id.as_str(),
                    trigger = trigger.as_str(),
                    recorded_count,
                    receivers,
                    "registry event worker reacted"
                );
                wake_request
            }
            Err(err) => {
                error!(
                    worker = id.as_str(),
                    trigger = trigger.as_str(),
                    error = %err,
                    "registry event worker failed"
                );
                WakeRequest::None
            }
        }
    }
}

/// Runs one journal-backed projector on the shared wake-driven loop.
pub(crate) struct JournalWorker<S, P> {
    db: S,
    projector: P,
    clock: Arc<dyn Clock>,
}

impl<S, P> JournalWorker<S, P> {
    pub fn new(db: S, projector: P, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            projector,
            clock,
        }
    }
}

impl<S, P> EventWorker for JournalWorker<S, P>
where
    S: FeedRegistryDb,
    P: Projector<S>,
    for<'tx> S::Tx<'tx>: EventJournalAppend,
{
    fn id(&self) -> WorkerId {
        self.projector.id().into()
    }

    fn interests(&self) -> EventInterests {
        self.projector.interests()
    }

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<Reaction> {
        let processor_id = self.projector.id();
        let interests = self.projector.interests();
        let mut tx = self.db.begin().await?;
        let batch = ReadInputBatch::<P::Input>::read(&mut tx, processor_id, interests).await?;

        let produced = self.projector.project_batch(&mut tx, batch.inputs).await?;
        let mut recorded_events = RecordedEvents::with_capacity(produced.len());
        {
            let mut event_recorder =
                EventRecorder::new(&mut tx, &mut recorded_events, self.clock.as_ref());
            event_recorder.record_all(produced).await?;
        }

        tx.advance_cursor(&batch.scanned_cursor).await?;
        tx.commit().await?;

        debug!(
            worker = self.id().as_str(),
            event_count = batch.event_count,
            cursor_position = ?batch.scanned_cursor.position(),
            "registry journal worker advanced"
        );

        Ok(Reaction::done(recorded_events))
    }
}

/// Runs one post-commit sink on the shared wake-driven loop.
pub(crate) struct PostCommitWorker<S, P> {
    db: S,
    processor: P,
}

impl<S, P> PostCommitWorker<S, P> {
    pub fn new(db: S, processor: P) -> Self {
        Self { db, processor }
    }
}

impl<S, P> EventWorker for PostCommitWorker<S, P>
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

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<Reaction> {
        let processor_id = self.processor.id();
        let interests = self.processor.interests();
        let mut tx = self.db.begin().await?;
        let batch = ReadInputBatch::<P::Input>::read(&mut tx, processor_id, interests).await?;

        tx.advance_cursor(&batch.scanned_cursor).await?;
        tx.commit().await?;

        for input in batch.inputs.into_inputs() {
            self.processor.sink(input).await;
        }

        debug!(
            worker = self.id().as_str(),
            event_count = batch.event_count,
            cursor_position = ?batch.scanned_cursor.position(),
            "registry post-commit worker advanced"
        );

        Ok(Reaction::done(RecordedEvents::empty()))
    }
}

/// One journal read decoded for a processor: the typed inputs plus the
/// cursor position the read scanned up to.
struct ReadInputBatch<I> {
    inputs: InputBatch<I>,
    scanned_cursor: EventCursor,
    event_count: usize,
}

impl<I> ReadInputBatch<I>
where
    I: EventInput,
{
    /// Loads the processor cursor, reads interested events after it, and
    /// decodes them into typed inputs. Undecodable events are handled by the
    /// shared failure policy.
    async fn read<Tx>(
        tx: &mut Tx,
        processor_id: ProcessorId,
        interests: EventInterests,
    ) -> WorkerResult<Self>
    where
        Tx: EventJournal + Send,
    {
        let cursor = tx.load_cursor(processor_id).await?;
        let batch = tx.read_after(&cursor, interests).await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();

        let mut inputs = Vec::with_capacity(event_count);
        for journaled in batch.into_events() {
            let occurred_at = journaled.occurred_at();
            match I::from_event(journaled.into_event(), occurred_at) {
                Ok(input) => inputs.push(input),
                Err(err) => err.skip_permanent(processor_id, "event input")?,
            }
        }

        Ok(Self {
            inputs: InputBatch::new(inputs),
            scanned_cursor,
            event_count,
        })
    }
}
