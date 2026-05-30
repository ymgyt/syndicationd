use super::{EncodedEvent, EventEncodingResult, EventPayload, encode_payload, event_type};
use crate::event::{
    Event, RequestEvent, RequestEventKind, SubscribeFeedRejected, SubscribeFeedRequested,
    UnsubscribeFeedRejected, UnsubscribeFeedRequested,
};

impl RequestEvent {
    pub(super) fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::SubscribeFeedRequested(event) => encode_payload(event),
            Self::SubscribeFeedRejected(event) => encode_payload(event),
            Self::UnsubscribeFeedRequested(event) => encode_payload(event),
            Self::UnsubscribeFeedRejected(event) => encode_payload(event),
        }
    }
}

impl RequestEventKind {
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::SubscribeFeedRequested => <SubscribeFeedRequested as EventPayload>::EVENT_TYPE,
            Self::SubscribeFeedRejected => <SubscribeFeedRejected as EventPayload>::EVENT_TYPE,
            Self::UnsubscribeFeedRequested => {
                <UnsubscribeFeedRequested as EventPayload>::EVENT_TYPE
            }
            Self::UnsubscribeFeedRejected => <UnsubscribeFeedRejected as EventPayload>::EVENT_TYPE,
        }
    }
}

impl EventPayload for SubscribeFeedRequested {
    const EVENT_TYPE: &'static str = event_type::REQUEST_SUBSCRIBE_FEED_REQUESTED;

    fn into_event(self) -> Event {
        Event::Request(RequestEvent::SubscribeFeedRequested(self))
    }
}

impl EventPayload for SubscribeFeedRejected {
    const EVENT_TYPE: &'static str = event_type::REQUEST_SUBSCRIBE_FEED_REJECTED;

    fn into_event(self) -> Event {
        Event::Request(RequestEvent::SubscribeFeedRejected(self))
    }
}

impl EventPayload for UnsubscribeFeedRequested {
    const EVENT_TYPE: &'static str = event_type::REQUEST_UNSUBSCRIBE_FEED_REQUESTED;

    fn into_event(self) -> Event {
        Event::Request(RequestEvent::UnsubscribeFeedRequested(self))
    }
}

impl EventPayload for UnsubscribeFeedRejected {
    const EVENT_TYPE: &'static str = event_type::REQUEST_UNSUBSCRIBE_FEED_REJECTED;

    fn into_event(self) -> Event {
        Event::Request(RequestEvent::UnsubscribeFeedRejected(self))
    }
}
