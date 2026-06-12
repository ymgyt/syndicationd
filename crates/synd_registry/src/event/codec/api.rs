use super::{EncodedEvent, EventEncodingResult, EventPayload, encode_payload, event_type};
use crate::event::{
    ApiEvent, ApiEventKind, ApiFeedSubscribeRejected, ApiFeedSubscribed,
    ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed,
    ApiTimelineChanged, Event,
};

impl ApiEvent {
    pub(super) fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::FeedSubscribed(event) => encode_payload(event),
            Self::FeedSubscribeRejected(event) => encode_payload(event),
            Self::FeedSubscriptionChanged(event) => encode_payload(event),
            Self::FeedUnsubscribed(event) => encode_payload(event),
            Self::FeedUnsubscribeRejected(event) => encode_payload(event),
            Self::TimelineChanged(event) => encode_payload(event),
        }
    }
}

impl ApiEventKind {
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::FeedSubscribed => <ApiFeedSubscribed as EventPayload>::EVENT_TYPE,
            Self::FeedSubscribeRejected => <ApiFeedSubscribeRejected as EventPayload>::EVENT_TYPE,
            Self::FeedSubscriptionChanged => {
                <ApiFeedSubscriptionChanged as EventPayload>::EVENT_TYPE
            }
            Self::FeedUnsubscribed => <ApiFeedUnsubscribed as EventPayload>::EVENT_TYPE,
            Self::FeedUnsubscribeRejected => {
                <ApiFeedUnsubscribeRejected as EventPayload>::EVENT_TYPE
            }
            Self::TimelineChanged => <ApiTimelineChanged as EventPayload>::EVENT_TYPE,
        }
    }
}

impl EventPayload for ApiFeedSubscribed {
    const EVENT_TYPE: &'static str = event_type::API_FEED_SUBSCRIBED;

    fn into_event(self) -> Event {
        Event::Api(ApiEvent::FeedSubscribed(self))
    }
}

impl EventPayload for ApiFeedSubscribeRejected {
    const EVENT_TYPE: &'static str = event_type::API_FEED_SUBSCRIBE_REJECTED;

    fn into_event(self) -> Event {
        Event::Api(ApiEvent::FeedSubscribeRejected(self))
    }
}

impl EventPayload for ApiFeedSubscriptionChanged {
    const EVENT_TYPE: &'static str = event_type::API_FEED_SUBSCRIPTION_CHANGED;

    fn into_event(self) -> Event {
        Event::Api(ApiEvent::FeedSubscriptionChanged(self))
    }
}

impl EventPayload for ApiFeedUnsubscribed {
    const EVENT_TYPE: &'static str = event_type::API_FEED_UNSUBSCRIBED;

    fn into_event(self) -> Event {
        Event::Api(ApiEvent::FeedUnsubscribed(self))
    }
}

impl EventPayload for ApiFeedUnsubscribeRejected {
    const EVENT_TYPE: &'static str = event_type::API_FEED_UNSUBSCRIBE_REJECTED;

    fn into_event(self) -> Event {
        Event::Api(ApiEvent::FeedUnsubscribeRejected(self))
    }
}

impl EventPayload for ApiTimelineChanged {
    const EVENT_TYPE: &'static str = event_type::API_TIMELINE_CHANGED;

    fn into_event(self) -> Event {
        Event::Api(ApiEvent::TimelineChanged(self))
    }
}
