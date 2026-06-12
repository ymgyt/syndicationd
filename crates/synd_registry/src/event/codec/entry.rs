use super::{EncodedEvent, EventEncodingResult, EventPayload, encode_payload, event_type};
use crate::event::{EntryChangedEvent, EntryDiscoveredEvent, EntryEvent, EntryEventKind, Event};

impl EntryEvent {
    pub(super) fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::Discovered(event) => encode_payload(event),
            Self::Changed(event) => encode_payload(event),
        }
    }
}

impl EntryEventKind {
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::Discovered => <EntryDiscoveredEvent as EventPayload>::EVENT_TYPE,
            Self::Changed => <EntryChangedEvent as EventPayload>::EVENT_TYPE,
        }
    }
}

impl EventPayload for EntryDiscoveredEvent {
    const EVENT_TYPE: &'static str = event_type::ENTRY_DISCOVERED;

    fn into_event(self) -> Event {
        Event::Entry(EntryEvent::Discovered(self))
    }
}

impl EventPayload for EntryChangedEvent {
    const EVENT_TYPE: &'static str = event_type::ENTRY_CHANGED;

    fn into_event(self) -> Event {
        Event::Entry(EntryEvent::Changed(self))
    }
}
