use super::{EncodedEvent, EventEncodingResult, EventPayload, encode_payload, event_type};
use crate::event::{
    Event, FeedSubscribedEvent, FeedUnsubscribedEvent, SubEvent, SubEventKind,
    SubscriptionChangedEvent,
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
            Self::FeedSubscribed => <FeedSubscribedEvent as EventPayload>::EVENT_TYPE,
            Self::SubscriptionChanged => <SubscriptionChangedEvent as EventPayload>::EVENT_TYPE,
            Self::FeedUnsubscribed => <FeedUnsubscribedEvent as EventPayload>::EVENT_TYPE,
        }
    }
}

impl EventPayload for FeedSubscribedEvent {
    const EVENT_TYPE: &'static str = event_type::SUB_FEED_SUBSCRIBED;

    fn into_event(self) -> Event {
        Event::Sub(SubEvent::FeedSubscribed(self))
    }
}

impl EventPayload for SubscriptionChangedEvent {
    const EVENT_TYPE: &'static str = event_type::SUB_SUBSCRIPTION_CHANGED;

    fn into_event(self) -> Event {
        Event::Sub(SubEvent::SubscriptionChanged(self))
    }
}

impl EventPayload for FeedUnsubscribedEvent {
    const EVENT_TYPE: &'static str = event_type::SUB_FEED_UNSUBSCRIBED;

    fn into_event(self) -> Event {
        Event::Sub(SubEvent::FeedUnsubscribed(self))
    }
}
