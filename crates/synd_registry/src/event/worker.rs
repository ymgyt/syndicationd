use std::{fmt, future::Future, time::Duration};

use thiserror::Error;
use tokio::{sync::broadcast, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    db::{CommitTx, FeedRegistryDb},
    error::RegistryDbError,
    event::{
        ConsumeContext, Consumer, EventCursor, JournalTx, PostCommit, Processor, ProcessorError,
        ProcessorId, ProcessorResult, RecordedEvents, Sink, Transactional,
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

/// Owns the task running one event processor.
#[derive(Debug)]
pub struct WorkerHandle {
    processor: ProcessorId,
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

impl WorkerHandle {
    pub fn new(processor: ProcessorId, task: JoinHandle<()>) -> Self {
        Self { processor, task }
    }

    pub fn processor(&self) -> ProcessorId {
        self.processor
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

    pub async fn join(mut self) -> Vec<(ProcessorId, Result<(), tokio::task::JoinError>)> {
        let handles = std::mem::take(&mut self.handles);
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let processor = handle.processor();
            results.push((processor, handle.join().await));
        }
        results
    }
}

impl Drop for WorkerSet {
    fn drop(&mut self) {
        self.abort();
    }
}

#[derive(Debug)]
pub(crate) struct ProcessReport {
    event_count: usize,
    scanned_cursor: EventCursor,
    recorded: RecordedEvents,
}

/// Runs one event processor as an asynchronous journal worker.
pub(crate) struct Worker<S, P> {
    db: S,
    processor: P,
    wake_publisher: EventWakePublisher,
    wake_subscriber: EventWakeSubscriber,
    poll_interval: Duration,
}

impl<S, P> Worker<S, P> {
    pub fn new(
        db: S,
        processor: P,
        wake_publisher: EventWakePublisher,
        wake_subscriber: EventWakeSubscriber,
        poll_interval: Duration,
    ) -> Self {
        Self {
            db,
            processor,
            wake_publisher,
            wake_subscriber,
            poll_interval,
        }
    }
}

impl<S, P> Worker<S, P>
where
    S: FeedRegistryDb,
    P: Processor,
    P::Phase: WorkerPhase<S, P>,
{
    pub fn spawn(self, ct: CancellationToken) -> WorkerHandle {
        let processor = self.processor.id();
        WorkerHandle::new(processor, tokio::spawn(self.run(ct)))
    }

    async fn run(mut self, ct: CancellationToken) {
        let processor = self.processor.id();
        debug!(
            processor = processor.as_str(),
            "registry event worker started"
        );
        self.process(Trigger::Startup).await;

        let mut interval = tokio::time::interval(self.poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval.tick().await;

        loop {
            tokio::select! {
                () = ct.cancelled() => break,
                wake = self.wake_subscriber.recv() => {
                    match wake {
                        Ok(recorded) if self.processor.interests().matches_any(recorded.kinds()) => {
                            self.process(Trigger::Wake).await;
                        }
                        Ok(_) => {}
                        Err(EventWakeRecvError::Lagged(skipped)) => {
                            warn!(
                                processor = processor.as_str(),
                                skipped,
                                "registry event worker wake lagged"
                            );
                            self.process(Trigger::WakeLagged).await;
                        }
                        Err(EventWakeRecvError::Closed) => {
                            warn!(
                                processor = processor.as_str(),
                                "registry event worker wake channel closed"
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
            processor = processor.as_str(),
            "registry event worker stopped"
        );
    }

    async fn process(&mut self, trigger: Trigger) {
        let processor = self.processor.id();
        match P::Phase::process(&self.db, &mut self.processor).await {
            Ok(report) => {
                debug!(
                    processor = processor.as_str(),
                    trigger = trigger.as_str(),
                    event_count = report.event_count,
                    recorded_count = report.recorded.len(),
                    cursor_position = ?report.scanned_cursor.position(),
                    "registry event worker processed events"
                );
                self.publish_recorded(trigger, report.recorded);
            }
            Err(err) => {
                error!(
                    processor = processor.as_str(),
                    trigger = trigger.as_str(),
                    error = %err,
                    "registry event worker processing failed"
                );
            }
        }
    }

    fn publish_recorded(&self, trigger: Trigger, recorded: RecordedEvents) {
        let recorded_count = recorded.len();
        let receivers = self.wake_publisher.publish(recorded);
        if recorded_count > 0 {
            let processor = self.processor.id();
            debug!(
                processor = processor.as_str(),
                trigger = trigger.as_str(),
                recorded_count,
                receivers,
                "registry event worker published recorded events"
            );
        }
    }
}

pub(crate) fn spawn_worker<S, P>(
    db: S,
    wake_publisher: EventWakePublisher,
    poll_interval: Duration,
    ct: CancellationToken,
    processor: P,
) -> WorkerHandle
where
    S: FeedRegistryDb,
    P: Processor,
    P::Phase: WorkerPhase<S, P>,
{
    let wake_subscriber = wake_publisher.subscribe();

    Worker::new(
        db,
        processor,
        wake_publisher,
        wake_subscriber,
        poll_interval,
    )
    .spawn(ct)
}

pub(crate) trait WorkerPhase<S, P>: Sized
where
    S: FeedRegistryDb,
    P: Processor<Phase = Self>,
{
    fn process(
        db: &S,
        processor: &mut P,
    ) -> impl Future<Output = WorkerResult<ProcessReport>> + Send;
}

impl<S, P> WorkerPhase<S, P> for Transactional
where
    S: FeedRegistryDb,
    P: Consumer<S, Phase = Transactional>,
{
    async fn process(db: &S, processor: &mut P) -> WorkerResult<ProcessReport> {
        let mut tx = db.begin().await?;
        let cursor = tx.load_cursor(processor.id()).await?;
        let batch = tx.read_after(&cursor, processor.interests()).await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();
        let mut cx = ConsumeContext::with_capacity(&mut tx, event_count);

        for journaled in batch.into_events() {
            let input = P::Input::try_from(journaled.into_event())?;
            processor.consume(&mut cx, input).await?;
        }
        let recorded = cx.into_recorded();

        tx.advance_cursor(&scanned_cursor).await?;
        tx.commit().await?;

        Ok(ProcessReport {
            event_count,
            scanned_cursor,
            recorded,
        })
    }
}

impl<S, P> WorkerPhase<S, P> for PostCommit
where
    S: FeedRegistryDb,
    P: Sink<Phase = PostCommit>,
{
    async fn process(db: &S, processor: &mut P) -> WorkerResult<ProcessReport> {
        let mut tx = db.begin().await?;
        let cursor = tx.load_cursor(processor.id()).await?;
        let batch = tx.read_after(&cursor, processor.interests()).await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();
        let inputs = batch
            .into_events()
            .into_iter()
            .map(|journaled| P::Input::try_from(journaled.into_event()))
            .collect::<ProcessorResult<Vec<_>>>()?;

        tx.advance_cursor(&scanned_cursor).await?;
        tx.commit().await?;

        for input in inputs {
            processor.consume(input).await?;
        }

        Ok(ProcessReport {
            event_count,
            scanned_cursor,
            recorded: RecordedEvents::empty(),
        })
    }
}
