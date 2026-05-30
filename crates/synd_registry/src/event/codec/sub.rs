use super::{EncodedEvent, EventEncodingResult, EventPayload, encode_payload, event_type};
use crate::event::{
    Event, FeedSubscribed, FeedUnsubscribed, SubEvent, SubEventKind, SubscriptionChanged,
};

impl SubEvent {
    pub(super) fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::FeedSubscribed(event) => encode_payload(event),
            Self::SubscriptionChanged(event) => encode_payload(event),
            Self::FeedUnsubscribed(event) => encode_payload(event),
        }
    }
}

impl SubEventKind {
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::FeedSubscribed => <FeedSubscribed as EventPayload>::EVENT_TYPE,
            Self::SubscriptionChanged => <SubscriptionChanged as EventPayload>::EVENT_TYPE,
            Self::FeedUnsubscribed => <FeedUnsubscribed as EventPayload>::EVENT_TYPE,
        }
    }
}

impl EventPayload for FeedSubscribed {
    const EVENT_TYPE: &'static str = event_type::SUB_FEED_SUBSCRIBED;

    fn into_event(self) -> Event {
        Event::Sub(SubEvent::FeedSubscribed(self))
    }
}

impl EventPayload for SubscriptionChanged {
    const EVENT_TYPE: &'static str = event_type::SUB_SUBSCRIPTION_CHANGED;

    fn into_event(self) -> Event {
        Event::Sub(SubEvent::SubscriptionChanged(self))
    }
}

impl EventPayload for FeedUnsubscribed {
    const EVENT_TYPE: &'static str = event_type::SUB_FEED_UNSUBSCRIBED;

    fn into_event(self) -> Event {
        Event::Sub(SubEvent::FeedUnsubscribed(self))
    }
}
