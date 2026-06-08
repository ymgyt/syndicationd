use super::{EncodedEvent, EventEncodingResult, EventPayload, encode_payload, event_type};
use crate::event::{Event, FeedChangedEvent, FeedDiscoveredEvent, FeedEvent, FeedEventKind};

impl FeedEvent {
    pub(super) fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::Discovered(event) => encode_payload(event),
            Self::Changed(event) => encode_payload(event),
        }
    }
}

impl FeedEventKind {
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::Discovered => <FeedDiscoveredEvent as EventPayload>::EVENT_TYPE,
            Self::Changed => <FeedChangedEvent as EventPayload>::EVENT_TYPE,
        }
    }
}

impl EventPayload for FeedDiscoveredEvent {
    const EVENT_TYPE: &'static str = event_type::FEED_DISCOVERED;

    fn into_event(self) -> Event {
        Event::Feed(FeedEvent::Discovered(self))
    }
}

impl EventPayload for FeedChangedEvent {
    const EVENT_TYPE: &'static str = event_type::FEED_CHANGED;

    fn into_event(self) -> Event {
        Event::Feed(FeedEvent::Changed(self))
    }
}
