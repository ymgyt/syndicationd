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
        EventInput, EventInterests, EventJournal, EventJournalAppend, EventReadBatch,
        EventRecorder, InputBatch, JournalHandler, Processor, ProcessorError, ProcessorId,
        Reaction, RecordedEvents, Sink, WakeRequest, skip_permanent_error,
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
    CrawlWorkerPool,
}

impl WorkerId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Processor(processor) => processor.as_str(),
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
    ) -> impl Future<Output = WorkerResult<Reaction<RecordedEvents>>> + Send;
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
    let mut requested_wake = process_event_worker(&mut worker, &wake, Trigger::Startup).await;

    let mut interval = tokio::time::interval(poll_interval);
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    interval.tick().await;

    loop {
        let requested_wake_timer = wait_for_wake_request(requested_wake);
        tokio::pin!(requested_wake_timer);
        tokio::select! {
            () = ct.cancelled() => break,
            () = &mut requested_wake_timer => {
                requested_wake = process_event_worker(&mut worker, &wake, Trigger::ScheduledWake).await;
            }
            received = wake.recv() => {
                match received {
                    Ok(recorded) if worker.interests().matches_any(recorded.types()) => {
                        requested_wake = process_event_worker(&mut worker, &wake, Trigger::Wake).await;
                    }
                    Ok(_) => {}
                    Err(EventWakeRecvError::Lagged(skipped)) => {
                        warn!(
                            worker = id.as_str(),
                            skipped,
                            "registry event worker wake lagged"
                        );
                        requested_wake = process_event_worker(&mut worker, &wake, Trigger::WakeLagged).await;
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
                requested_wake = process_event_worker(&mut worker, &wake, Trigger::Poll).await;
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
async fn process_event_worker<W>(worker: &mut W, wake: &EventWake, trigger: Trigger) -> WakeRequest
where
    W: EventWorker,
{
    let id = worker.id();
    match worker.react(trigger).await {
        Ok(reaction) => {
            let (recorded, wake_request) = reaction.into_parts();
            let recorded_count = recorded.len();
            let receivers = wake.publish(recorded);
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

async fn wait_for_wake_request(wake_request: WakeRequest) {
    match wake_request {
        WakeRequest::None => std::future::pending().await,
        WakeRequest::At(wake_at) => tokio::time::sleep_until(wake_instant(wake_at)).await,
    }
}

fn wake_instant(wake_at: DateTime<Utc>) -> tokio::time::Instant {
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

/// Runs one journal-backed handler on the shared wake-driven loop.
pub(crate) struct JournalWorker<S, P> {
    db: S,
    processor: P,
    clock: Arc<dyn Clock>,
}

impl<S, P> JournalWorker<S, P> {
    pub fn new(db: S, processor: P, clock: Arc<dyn Clock>) -> Self {
        Self {
            db,
            processor,
            clock,
        }
    }
}

impl<S, P> EventWorker for JournalWorker<S, P>
where
    S: FeedRegistryDb,
    P: JournalHandler<S>,
    for<'tx> S::Tx<'tx>: EventJournalAppend,
{
    fn id(&self) -> WorkerId {
        self.processor.id().into()
    }

    fn interests(&self) -> EventInterests {
        self.processor.interests()
    }

    async fn react(&mut self, trigger: Trigger) -> WorkerResult<Reaction<RecordedEvents>> {
        let processor_id = self.processor.id();
        let mut tx = self.db.begin().await?;
        let cursor = tx.load_cursor(processor_id).await?;
        let batch = tx.read_after(&cursor, self.processor.interests()).await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();

        let inputs = collect_inputs::<P>(processor_id, batch)?;
        let reaction = self
            .processor
            .handle_journal_batch(&mut tx, self.clock.now(), trigger, InputBatch::new(inputs))
            .await?;
        let (produced, wake_request) = reaction.into_parts();
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
            "registry journal worker advanced"
        );

        Ok(Reaction::new(recorded_events, wake_request))
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

    async fn react(&mut self, _trigger: Trigger) -> WorkerResult<Reaction<RecordedEvents>> {
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
            self.processor.sink(input).await;
        }

        debug!(
            worker = self.id().as_str(),
            event_count,
            cursor_position = ?scanned_cursor.position(),
            "registry post-commit worker advanced"
        );

        Ok(Reaction::done(RecordedEvents::empty()))
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
