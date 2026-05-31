use std::{fmt, time::Duration};

use thiserror::Error;
use tokio::{sync::broadcast, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use crate::event::{
    ConsumerEventInput, EventConsumer, EventConsumerError, EventConsumerId, EventCursor,
    EventJournal, EventJournalError, RecordedEvents,
};

pub type WorkerResult<T> = Result<T, WorkerError>;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error(transparent)]
    Journal(#[from] EventJournalError),
    #[error(transparent)]
    Consumer(#[from] EventConsumerError),
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventWakeRecvError {
    Closed,
    Lagged(u64),
}

#[derive(Clone)]
pub struct EventWakePublisher {
    sender: broadcast::Sender<RecordedEvents>,
}

impl fmt::Debug for EventWakePublisher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventWakePublisher").finish_non_exhaustive()
    }
}

pub struct EventWakeSubscriber {
    receiver: broadcast::Receiver<RecordedEvents>,
}

#[derive(Debug)]
pub struct WorkerHandle {
    consumer: EventConsumerId,
    task: JoinHandle<()>,
}

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
    pub fn new(consumer: EventConsumerId, task: JoinHandle<()>) -> Self {
        Self { consumer, task }
    }

    pub fn consumer(&self) -> EventConsumerId {
        self.consumer
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

    pub async fn join(mut self) -> Vec<(EventConsumerId, Result<(), tokio::task::JoinError>)> {
        let handles = std::mem::take(&mut self.handles);
        let mut results = Vec::with_capacity(handles.len());
        for handle in handles {
            let consumer = handle.consumer();
            results.push((consumer, handle.join().await));
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
pub struct Worker<J, C> {
    journal: J,
    consumer: C,
    wake_publisher: EventWakePublisher,
    wake_subscriber: EventWakeSubscriber,
    poll_interval: Duration,
}

impl<J, C> Worker<J, C>
where
    J: EventJournal,
    C: EventConsumer,
{
    pub fn new(
        journal: J,
        consumer: C,
        wake_publisher: EventWakePublisher,
        wake_subscriber: EventWakeSubscriber,
        poll_interval: Duration,
    ) -> Self {
        Self {
            journal,
            consumer,
            wake_publisher,
            wake_subscriber,
            poll_interval,
        }
    }

    pub async fn run(mut self, ct: CancellationToken) {
        let consumer = self.consumer.id();
        debug!(
            consumer = consumer.as_str(),
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
                        Ok(recorded) if self.consumer.read_filter().matches_any(recorded.kinds()) => {
                            self.process(Trigger::Wake).await;
                        }
                        Ok(_) => {}
                        Err(EventWakeRecvError::Lagged(skipped)) => {
                            warn!(
                                consumer = consumer.as_str(),
                                skipped,
                                "registry event worker wake lagged"
                            );
                            self.process(Trigger::WakeLagged).await;
                        }
                        Err(EventWakeRecvError::Closed) => {
                            warn!(
                                consumer = consumer.as_str(),
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
            consumer = consumer.as_str(),
            "registry event worker stopped"
        );
    }

    pub fn spawn(self, ct: CancellationToken) -> WorkerHandle {
        let consumer = self.consumer.id();
        WorkerHandle::new(consumer, tokio::spawn(self.run(ct)))
    }

    async fn process(&mut self, trigger: Trigger) {
        let consumer = self.consumer.id();
        match self.drain().await {
            Ok(outcome) => {
                debug!(
                    consumer = consumer.as_str(),
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
                    consumer = consumer.as_str(),
                    trigger = trigger.as_str(),
                    error = %err,
                    "registry event worker drain failed"
                );
            }
        }
    }

    async fn drain(&mut self) -> WorkerResult<DrainOutcome> {
        let consumer = self.consumer.id();
        let cursor = self.journal.load_cursor(consumer).await?;
        let batch = self
            .journal
            .read_after(&cursor, self.consumer.read_filter())
            .await?;
        let event_count = batch.events().len();
        let scanned_cursor = batch.scanned_cursor().clone();
        let recorded = if !batch.is_empty()
            && let Some(input) = C::Input::from_batch(batch)?
        {
            self.consumer.consume(input).await?
        } else {
            RecordedEvents::empty()
        };
        self.journal.commit_cursor(&scanned_cursor).await?;
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
            debug!(
                consumer = self.consumer.id().as_str(),
                trigger = trigger.as_str(),
                recorded_count,
                receivers,
                "registry event worker published recorded events"
            );
        }
    }
}
