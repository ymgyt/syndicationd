use super::{EncodedEvent, EventEncodingResult, EventPayload, encode_payload, event_type};
use crate::event::{Event, TimelineChangedEvent, TimelineEvent, TimelineEventKind};

impl TimelineEvent {
    pub(super) fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::Changed(event) => encode_payload(event),
        }
    }
}

impl TimelineEventKind {
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::Changed => <TimelineChangedEvent as EventPayload>::EVENT_TYPE,
        }
    }
}

impl EventPayload for TimelineChangedEvent {
    const EVENT_TYPE: &'static str = event_type::TIMELINE_CHANGED;

    fn into_event(self) -> Event {
        Event::Timeline(TimelineEvent::Changed(self))
    }
}
