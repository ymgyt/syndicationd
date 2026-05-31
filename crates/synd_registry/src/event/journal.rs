use thiserror::Error;

use crate::event::{Event, EventInterests, ProcessorId};

pub type EventJournalResult<T> = Result<T, EventJournalError>;

#[derive(Debug, Error)]
pub enum EventJournalError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Stores registry events and reloads processor progress through the journal.
pub trait EventJournal: Clone + Send + Sync + 'static {
    /// Records that the event happened.
    fn append(&self, event: Event) -> impl Future<Output = EventJournalResult<()>> + Send;

    /// Reads interested entries after the supplied cursor.
    fn read_after(
        &self,
        cursor: &EventCursor,
        interests: EventInterests,
    ) -> impl Future<Output = EventJournalResult<EventReadBatch>> + Send;

    /// Loads the cursor for a processor, or `EventCursor::initial()` for a new processor.
    fn load_cursor(
        &self,
        processor: ProcessorId,
    ) -> impl Future<Output = EventJournalResult<EventCursor>> + Send;
}

/// A batch of journal entries selected for one processor.
///
/// `events` contains only entries the processor should handle. `scanned_cursor`
/// marks the journal position that was inspected, including entries that did
/// not belong to the processor.
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

/// A registry event returned from the journal with its journal position.
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

/// A processor's read cursor in the event journal.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventCursor {
    processor: ProcessorId,
    position: EventCursorPos,
}

impl EventCursor {
    pub fn initial(processor: ProcessorId) -> Self {
        Self {
            processor,
            position: EventCursorPos::initial(),
        }
    }

    pub fn at(processor: ProcessorId, position: EventCursorPos) -> Self {
        Self {
            processor,
            position,
        }
    }

    pub fn processor(&self) -> ProcessorId {
        self.processor
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
