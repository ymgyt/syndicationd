use std::future::Future;

use thiserror::Error;

use crate::event::{EventConsumerId, EventReadFilter, RegistryEvent};

pub type EventJournalResult<T> = Result<T, EventJournalError>;

#[derive(Debug, Error)]
pub enum EventJournalError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Appends registry events and tracks where each event consumer has read up to.
pub trait EventJournal: Clone + Send + Sync + 'static {
    /// Records that the event happened.
    async fn append(&self, event: RegistryEvent) -> EventJournalResult<()>;

    /// Reads filtered entries after the supplied cursor.
    async fn read_after(
        &self,
        cursor: &EventCursor,
        filter: EventReadFilter,
    ) -> EventJournalResult<EventReadBatch>;

    /// Loads the cursor for a consumer, or `EventCursor::initial()` for a new consumer.
    async fn load_cursor(&self, consumer: EventConsumerId) -> EventJournalResult<EventCursor>;

    /// Records that the cursor's consumer has fully processed events through the cursor.
    async fn commit_cursor(&self, cursor: &EventCursor) -> EventJournalResult<()>;
}

pub trait EventJournalExt: EventJournal {
    fn consumer(
        &self,
        consumer: EventConsumerId,
        filter: EventReadFilter,
    ) -> EventJournalConsumer<Self>
    where
        Self: Sized,
    {
        EventJournalConsumer::new(self.clone(), consumer, filter)
    }
}

impl<J> EventJournalExt for J where J: EventJournal {}

/// A journal view bound to one event consumer.
///
/// This type owns the cursor workflow for a consumer: load the current cursor,
/// read relevant pending events, process them in order, and commit progress
/// only after it is safe to record.
#[derive(Debug, Clone)]
pub struct EventJournalConsumer<J> {
    journal: J,
    consumer: EventConsumerId,
    filter: EventReadFilter,
}

impl<J> EventJournalConsumer<J>
where
    J: EventJournal,
{
    pub fn new(journal: J, consumer: EventConsumerId, filter: EventReadFilter) -> Self {
        Self {
            journal,
            consumer,
            filter,
        }
    }

    pub fn consumer(&self) -> EventConsumerId {
        self.consumer
    }

    /// Process pending events for this consumer in journal order.
    ///
    /// The cursor is committed after each event handler returns `Ok(())`.
    /// If a handler returns an error, processing stops and that event's cursor
    /// is not committed.
    pub async fn process_pending<F, Fut>(&self, mut handle_event: F) -> EventJournalResult<usize>
    where
        F: FnMut(JournaledEvent) -> Fut,
        Fut: Future<Output = EventJournalResult<()>>,
    {
        let cursor = self.journal.load_cursor(self.consumer).await?;
        let batch = self.journal.read_after(&cursor, self.filter).await?;
        let mut processed = 0;

        for event in batch.events {
            let cursor = event.cursor().clone();
            handle_event(event).await?;
            self.journal.commit_cursor(&cursor).await?;
            processed += 1;
        }

        self.journal.commit_cursor(&batch.scanned_cursor).await?;

        Ok(processed)
    }
}

/// A batch of journal entries selected for one consumer.
///
/// `events` contains only entries the consumer should handle. `scanned_cursor`
/// marks the journal position that was inspected, including entries that did
/// not belong to the consumer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventReadBatch {
    events: Vec<JournaledEvent>,
    scanned_cursor: EventCursor,
}

impl EventReadBatch {
    pub fn new(events: Vec<JournaledEvent>, scanned_cursor: EventCursor) -> Self {
        Self {
            events,
            scanned_cursor,
        }
    }

    pub fn empty(scanned_cursor: EventCursor) -> Self {
        Self::new(Vec::new(), scanned_cursor)
    }

    pub fn events(&self) -> &[JournaledEvent] {
        &self.events
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn into_events(self) -> Vec<JournaledEvent> {
        self.events
    }

    pub fn scanned_cursor(&self) -> &EventCursor {
        &self.scanned_cursor
    }
}

/// A registry event returned from the journal with a consumer-owned cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournaledEvent {
    cursor: EventCursor,
    event: RegistryEvent,
}

impl JournaledEvent {
    pub fn new(cursor: EventCursor, event: RegistryEvent) -> Self {
        Self { cursor, event }
    }

    pub fn cursor(&self) -> &EventCursor {
        &self.cursor
    }

    pub fn event(&self) -> &RegistryEvent {
        &self.event
    }

    pub fn into_event(self) -> RegistryEvent {
        self.event
    }
}

/// A consumer's read cursor in the event journal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventCursor {
    consumer: EventConsumerId,
    position: EventCursorPos,
}

impl EventCursor {
    pub fn initial(consumer: EventConsumerId) -> Self {
        Self {
            consumer,
            position: EventCursorPos::initial(),
        }
    }

    pub fn at(consumer: EventConsumerId, position: EventCursorPos) -> Self {
        Self { consumer, position }
    }

    pub fn consumer(&self) -> EventConsumerId {
        self.consumer
    }

    pub fn position(&self) -> &EventCursorPos {
        &self.position
    }
}

/// A position in the event journal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventCursorPos {
    Initial,
    Position(String),
}

impl EventCursorPos {
    pub fn initial() -> Self {
        Self::Initial
    }

    pub fn position(position: impl Into<String>) -> Self {
        Self::Position(position.into())
    }
}
