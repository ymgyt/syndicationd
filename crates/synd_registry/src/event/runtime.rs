use futures_util::future;
use thiserror::Error;

use crate::event::{
    ConsumerDispatch, ConsumerRegistry, EmptyConsumerRegistry, Event, EventConsumerError,
    EventConsumerId, EventConsumerSession, EventJournal, EventJournalError, RecordedEvents,
};

pub type EventRuntimeResult<T> = Result<T, EventRuntimeError>;

#[derive(Debug, Error)]
pub enum EventRuntimeError {
    #[error(transparent)]
    Journal(#[from] EventJournalError),
    #[error(transparent)]
    Consumer(#[from] EventConsumerError),
    #[error("event consumer is not registered: {0}")]
    UnknownConsumer(&'static str),
}

pub trait EventSubmitter: Clone + Send + Sync + 'static {
    async fn submit(&self, events: Vec<Event>) -> EventRuntimeResult<EventRuntimeOutput>;
}

/// Records submitted events into an event journal.
#[derive(Debug, Clone)]
pub struct EventRecorder<J> {
    journal: J,
}

impl<J> EventRecorder<J>
where
    J: EventJournal,
{
    pub fn new(journal: J) -> Self {
        Self { journal }
    }

    pub async fn record(&self, events: Vec<Event>) -> EventRuntimeResult<RecordedEvents> {
        let mut kinds = Vec::with_capacity(events.len());
        for event in events {
            kinds.push(event.kind());
            self.journal.append(event).await?;
        }
        Ok(RecordedEvents::new(kinds))
    }
}

/// Result of recording events and running interested consumers once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventRuntimeOutput {
    /// Events recorded from the runtime caller's submission.
    recorded: RecordedEvents,
    /// Events recorded by consumers while reacting to `recorded`.
    consumer_recorded: RecordedEvents,
}

impl EventRuntimeOutput {
    pub fn new(recorded: RecordedEvents, consumer_recorded: RecordedEvents) -> Self {
        Self {
            recorded,
            consumer_recorded,
        }
    }

    pub fn recorded(&self) -> &RecordedEvents {
        &self.recorded
    }

    pub fn consumer_recorded(&self) -> &RecordedEvents {
        &self.consumer_recorded
    }

    pub fn into_consumer_recorded(self) -> RecordedEvents {
        self.consumer_recorded
    }
}

/// Repeatedly reacts to events recorded by consumers until no new event is recorded.
#[derive(Debug, Clone)]
struct RecordedEventPropagation<J, C = EmptyConsumerRegistry> {
    journal: J,
    consumers: C,
    recorded: RecordedEvents,
}

impl<J, C> RecordedEventPropagation<J, C>
where
    J: EventJournal,
    C: ConsumerRegistry,
{
    fn new(journal: J, consumers: C, recorded: RecordedEvents) -> Self {
        Self {
            journal,
            consumers,
            recorded,
        }
    }

    async fn run(self) -> EventRuntimeResult<RecordedEvents> {
        let mut pending = self.recorded;
        let mut propagated = RecordedEvents::empty();

        while !pending.is_empty() {
            let consumer_recorded =
                ConsumerDispatchBatch::new(self.journal.clone(), self.consumers.clone(), pending)
                    .dispatch()
                    .await?;
            pending = consumer_recorded.clone();
            propagated.extend(consumer_recorded);
        }

        Ok(propagated)
    }
}

/// Dispatches a recorded batch to the consumers interested in that batch.
#[derive(Debug, Clone)]
pub struct ConsumerDispatchBatch<J, C = EmptyConsumerRegistry> {
    journal: J,
    consumers: C,
    recorded: RecordedEvents,
}

impl<J, C> ConsumerDispatchBatch<J, C>
where
    J: EventJournal,
    C: ConsumerRegistry,
{
    pub fn new(journal: J, consumers: C, recorded: RecordedEvents) -> Self {
        Self {
            journal,
            consumers,
            recorded,
        }
    }

    pub async fn dispatch(self) -> EventRuntimeResult<RecordedEvents> {
        let futures = self
            .consumers
            .interested_in(&self.recorded)
            .into_iter()
            .map(|consumer| self.dispatch_consumer(consumer));
        let recorded = future::try_join_all(futures).await?;
        Ok(collect_recorded_events(recorded))
    }

    async fn dispatch_consumer(
        &self,
        consumer: EventConsumerId,
    ) -> EventRuntimeResult<RecordedEvents> {
        let Some(filter) = self.consumers.read_filter(consumer) else {
            return Err(EventRuntimeError::UnknownConsumer(consumer.as_str()));
        };
        let cursor = self.journal.load_cursor(consumer).await?;
        let batch = self.journal.read_after(&cursor, filter).await?;
        let scanned_cursor = batch.scanned_cursor().clone();
        let mut session = EventConsumerSession::new(&self.journal);
        if !batch.is_empty() {
            let Some(dispatch) = self.consumers.dispatch(consumer) else {
                return Err(EventRuntimeError::UnknownConsumer(consumer.as_str()));
            };
            dispatch.consume(batch, &mut session).await?;
        }
        let recorded = session.into_recorded();
        self.journal.commit_cursor(&scanned_cursor).await?;
        Ok(recorded)
    }
}

/// Accepts submitted events into the journal and dispatches interested consumers once.
#[derive(Debug, Clone)]
pub struct EventRuntime<J, C = EmptyConsumerRegistry> {
    recorder: EventRecorder<J>,
    journal: J,
    consumers: C,
}

impl<J> EventRuntime<J, EmptyConsumerRegistry>
where
    J: EventJournal,
{
    pub fn new(journal: J) -> Self {
        Self::with_consumers(journal, EmptyConsumerRegistry)
    }
}

impl<J, C> EventRuntime<J, C>
where
    J: EventJournal,
    C: ConsumerRegistry,
{
    pub fn with_consumers(journal: J, consumers: C) -> Self {
        Self {
            recorder: EventRecorder::new(journal.clone()),
            journal,
            consumers,
        }
    }

    pub async fn react_to(
        &self,
        recorded: RecordedEvents,
    ) -> EventRuntimeResult<EventRuntimeOutput> {
        let consumer_recorded = RecordedEventPropagation::new(
            self.journal.clone(),
            self.consumers.clone(),
            recorded.clone(),
        )
        .run()
        .await?;
        Ok(EventRuntimeOutput::new(recorded, consumer_recorded))
    }
}

impl<J, C> EventSubmitter for EventRuntime<J, C>
where
    J: EventJournal,
    C: ConsumerRegistry,
{
    async fn submit(&self, events: Vec<Event>) -> EventRuntimeResult<EventRuntimeOutput> {
        let recorded = self.recorder.record(events).await?;
        self.react_to(recorded).await
    }
}

fn collect_recorded_events(recorded: Vec<RecordedEvents>) -> RecordedEvents {
    let mut collected = RecordedEvents::empty();
    for recorded in recorded {
        collected.extend(recorded);
    }
    collected
}
