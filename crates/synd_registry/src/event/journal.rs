use thiserror::Error;

use crate::event::{Event, EventConsumerId, EventReadFilter};

pub type EventJournalResult<T> = Result<T, EventJournalError>;

#[derive(Debug, Error)]
pub enum EventJournalError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Appends registry events and tracks where each event consumer has read up to.
pub trait EventJournal: Clone + Send + Sync + 'static {
    /// Records that the event happened.
    fn append(&self, event: Event) -> impl Future<Output = EventJournalResult<()>> + Send;

    /// Reads filtered entries after the supplied cursor.
    fn read_after(
        &self,
        cursor: &EventCursor,
        filter: EventReadFilter,
    ) -> impl Future<Output = EventJournalResult<EventReadBatch>> + Send;

    /// Loads the cursor for a consumer, or `EventCursor::initial()` for a new consumer.
    fn load_cursor(
        &self,
        consumer: EventConsumerId,
    ) -> impl Future<Output = EventJournalResult<EventCursor>> + Send;

    /// Records that the cursor's consumer has fully processed events through the cursor.
    fn commit_cursor(
        &self,
        cursor: &EventCursor,
    ) -> impl Future<Output = EventJournalResult<()>> + Send;
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
    event: Event,
}

impl JournaledEvent {
    pub fn new(cursor: EventCursor, event: Event) -> Self {
        Self { cursor, event }
    }

    pub fn cursor(&self) -> &EventCursor {
        &self.cursor
    }

    pub fn event(&self) -> &Event {
        &self.event
    }

    pub fn into_event(self) -> Event {
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
