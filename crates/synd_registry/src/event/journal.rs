use std::future::Future;

use chrono::{DateTime, Utc};

use crate::{
    error::RegistryDbResult,
    event::{Event, EventInterests, EventType, ProcessorId},
};

/// Transactional event journal append operation used by `EventRecorder`.
pub trait JournalAppendTx {
    /// Records that the event happened in the current transaction.
    fn append_event(
        &mut self,
        event: Event,
        occurred_at: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<EventType>> + Send;
}

/// Transactional event journal read and cursor operations.
pub trait JournalTx {
    /// Reads interested entries after the supplied cursor in the current transaction.
    fn read_after(
        &mut self,
        cursor: &EventCursor,
        interests: EventInterests,
    ) -> impl Future<Output = RegistryDbResult<EventReadBatch>> + Send;

    /// Loads the cursor for a processor, or `EventCursor::initial()` for a new processor.
    fn load_cursor(
        &mut self,
        processor: ProcessorId,
    ) -> impl Future<Output = RegistryDbResult<EventCursor>> + Send;

    /// Advances the processor cursor in the current transaction.
    fn advance_cursor(
        &mut self,
        cursor: &EventCursor,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;
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
    occurred_at: DateTime<Utc>,
}

impl JournaledEvent {
    pub fn new(cursor: EventCursor, event: Event, occurred_at: DateTime<Utc>) -> Self {
        Self {
            cursor,
            event,
            occurred_at,
        }
    }

    pub fn cursor(&self) -> &EventCursor {
        &self.cursor
    }

    pub fn event(&self) -> &Event {
        &self.event
    }

    pub fn occurred_at(&self) -> DateTime<Utc> {
        self.occurred_at
    }

    pub fn into_event(self) -> Event {
        self.event
    }

    pub fn into_parts(self) -> (EventCursor, Event, DateTime<Utc>) {
        (self.cursor, self.event, self.occurred_at)
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
