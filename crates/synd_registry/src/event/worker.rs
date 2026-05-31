use std::{fmt, time::Duration};

use thiserror::Error;
use tokio::{sync::broadcast, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::{
    db::{FeedRegistryDb, RegistryDbTransaction},
    error::RegistryDbError,
    event::{
        Consumer, EventCursor, EventJournal, EventJournalError, ProcessorError, ProcessorId,
        ProcessorResult, RecordedEvents, Sink,
    },
};

/// Result type returned by registry event workers.
pub type WorkerResult<T> = Result<T, WorkerError>;

/// Error returned while an event worker drains the journal.
#[derive(Debug, Error)]
pub enum WorkerError {
    #[error(transparent)]
    Journal(#[from] EventJournalError),
    #[error(transparent)]
    RegistryDb(#[from] RegistryDbError),
    #[error(transparent)]
    Processor(#[from] ProcessorError),
}

/// Source that caused an event worker drain attempt.
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

/// Result of one successful journal drain.
#[derive(Debug)]
pub struct DrainOutcome {
    event_count: usize,
    scanned_cursor: EventCursor,
    recorded: RecordedEvents,
}

impl DrainOutcome {
    pub fn event_count(&self) -> usize {
        self.event_count
    }

    pub fn scanned_cursor(&self) -> &EventCursor {
        &self.scanned_cursor
    }

    pub fn recorded(&self) -> &RecordedEvents {
        &self.recorded
    }

    pub fn into_recorded(self) -> RecordedEvents {
        self.recorded
    }
}

/// Runs one event consumer as an asynchronous journal worker.
pub struct Worker<S, J, C> {
    db: S,
    journal: J,
    consumer: C,
    wake_publisher: EventWakePublisher,
    wake_subscriber: EventWakeSubscriber,
    poll_interval: Duration,
}

impl<S, J, C> Worker<S, J, C>
where
    S: FeedRegistryDb,
    J: EventJournal,
    C: Consumer<S>,
{
    pub fn new(
        db: S,
        journal: J,
        consumer: C,
        wake_publisher: EventWakePublisher,
        wake_subscriber: EventWakeSubscriber,
        poll_interval: Duration,
    ) -> Self {
        Self {
            db,
            journal,
            consumer,
            wake_publisher,
            wake_subscriber,
            poll_interval,
        }
    }

    pub async fn run(mut self, ct: CancellationToken) {
        let processor = self.consumer.id();
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
                        Ok(recorded) if self.consumer.interests().matches_any(recorded.kinds()) => {
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

    pub fn spawn(self, ct: CancellationToken) -> WorkerHandle {
        let processor = self.consumer.id();
        WorkerHandle::new(processor, tokio::spawn(self.run(ct)))
    }

    async fn process(&mut self, trigger: Trigger) {
        let processor = self.consumer.id();
        match self.drain().await {
            Ok(outcome) => {
                debug!(
                    processor = processor.as_str(),
                    trigger = trigger.as_str(),
                    event_count = outcome.event_count(),
                    recorded_count = outcome.recorded().len(),
                    cursor_position = ?outcome.scanned_cursor().position(),
                    "registry event worker drained events"
                );
                self.publish_recorded(trigger, outcome.into_recorded());
            }
            Err(err) => {
                error!(
                    processor = processor.as_str(),
                    trigger = trigger.as_str(),
                    error = %err,
                    "registry event worker drain failed"
                );
            }
        }
    }

    async fn drain(&mut self) -> WorkerResult<DrainOutcome> {
        let processor = self.consumer.id();
        let cursor = self.journal.load_cursor(processor).await?;
        let batch = self
            .journal
            .read_after(&cursor, self.consumer.interests())
            .await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();
        let mut recorded = RecordedEvents::empty();
        let mut tx = self.db.begin().await?;

        for journaled in batch.into_events() {
            let input = C::Input::try_from(journaled.into_event())?;
            recorded.extend(self.consumer.consume(&mut tx, input).await?);
        }

        tx.advance_event_cursor(&scanned_cursor).await?;
        tx.commit().await?;

        Ok(DrainOutcome {
            event_count,
            scanned_cursor,
            recorded,
        })
    }

    fn publish_recorded(&self, trigger: Trigger, recorded: RecordedEvents) {
        let recorded_count = recorded.len();
        let receivers = self.wake_publisher.publish(recorded);
        if recorded_count > 0 {
            let processor = self.consumer.id();
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

/// Runs one terminal event sink as an asynchronous journal worker.
pub struct SinkWorker<S, J, K> {
    db: S,
    journal: J,
    sink: K,
    wake_subscriber: EventWakeSubscriber,
    poll_interval: Duration,
}

impl<S, J, K> SinkWorker<S, J, K>
where
    S: FeedRegistryDb,
    J: EventJournal,
    K: Sink,
{
    pub fn new(
        db: S,
        journal: J,
        sink: K,
        wake_subscriber: EventWakeSubscriber,
        poll_interval: Duration,
    ) -> Self {
        Self {
            db,
            journal,
            sink,
            wake_subscriber,
            poll_interval,
        }
    }

    pub async fn run(mut self, ct: CancellationToken) {
        let processor = self.sink.id();
        debug!(
            processor = processor.as_str(),
            "registry event sink worker started"
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
                        Ok(recorded) if self.sink.interests().matches_any(recorded.kinds()) => {
                            self.process(Trigger::Wake).await;
                        }
                        Ok(_) => {}
                        Err(EventWakeRecvError::Lagged(skipped)) => {
                            warn!(
                                processor = processor.as_str(),
                                skipped,
                                "registry event sink worker wake lagged"
                            );
                            self.process(Trigger::WakeLagged).await;
                        }
                        Err(EventWakeRecvError::Closed) => {
                            warn!(
                                processor = processor.as_str(),
                                "registry event sink worker wake channel closed"
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
            "registry event sink worker stopped"
        );
    }

    pub fn spawn(self, ct: CancellationToken) -> WorkerHandle {
        let processor = self.sink.id();
        WorkerHandle::new(processor, tokio::spawn(self.run(ct)))
    }

    async fn process(&mut self, trigger: Trigger) {
        let processor = self.sink.id();
        match self.drain().await {
            Ok(outcome) => {
                debug!(
                    processor = processor.as_str(),
                    trigger = trigger.as_str(),
                    event_count = outcome.event_count(),
                    cursor_position = ?outcome.scanned_cursor().position(),
                    "registry event sink worker drained events"
                );
            }
            Err(err) => {
                error!(
                    processor = processor.as_str(),
                    trigger = trigger.as_str(),
                    error = %err,
                    "registry event sink worker drain failed"
                );
            }
        }
    }

    async fn drain(&mut self) -> WorkerResult<DrainOutcome> {
        let processor = self.sink.id();
        let cursor = self.journal.load_cursor(processor).await?;
        let batch = self
            .journal
            .read_after(&cursor, self.sink.interests())
            .await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();
        let inputs = batch
            .into_events()
            .into_iter()
            .map(|journaled| K::Input::try_from(journaled.into_event()))
            .collect::<ProcessorResult<Vec<_>>>()?;

        let mut tx = self.db.begin().await?;
        tx.advance_event_cursor(&scanned_cursor).await?;
        tx.commit().await?;

        for input in inputs {
            self.sink.consume(input).await?;
        }

        Ok(DrainOutcome {
            event_count,
            scanned_cursor,
            recorded: RecordedEvents::empty(),
        })
    }
}
