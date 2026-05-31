use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

use super::domain::{
    ApiFeedSubscribeRejected, ApiFeedSubscribed, ApiFeedSubscriptionChanged,
    ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed, Event, EventKind, FeedSubscribed,
    FeedUnsubscribed, SubscribeFeedRejected, SubscribeFeedRequested, SubscriptionChanged,
    UnsubscribeFeedRejected, UnsubscribeFeedRequested,
};

mod api;
mod event_type;
mod request;
mod sub;

pub type EventEncodingResult<T> = Result<T, EventEncodingError>;

#[derive(Debug, Error)]
pub enum EventEncodingError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("unknown event type: {0}")]
    UnknownEventType(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedEvent {
    pub event_type: &'static str,
    pub payload_json: String,
}

pub trait EventPayload: Serialize + DeserializeOwned + Sized {
    const EVENT_TYPE: &'static str;

    fn into_event(self) -> Event;
}

pub trait EventEncoding: Sized {
    fn encode(&self) -> EventEncodingResult<EncodedEvent>;

    fn decode(event_type: &str, payload_json: &str) -> EventEncodingResult<Self>;
}

impl EventEncoding for Event {
    fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::Request(event) => event.encode(),
            Self::Sub(event) => event.encode(),
            Self::Api(event) => event.encode(),
        }
    }

    fn decode(event_type: &str, payload_json: &str) -> EventEncodingResult<Self> {
        match event_type {
            <SubscribeFeedRequested as EventPayload>::EVENT_TYPE => {
                decode_payload::<SubscribeFeedRequested>(payload_json).map(EventPayload::into_event)
            }
            <SubscribeFeedRejected as EventPayload>::EVENT_TYPE => {
                decode_payload::<SubscribeFeedRejected>(payload_json).map(EventPayload::into_event)
            }
            <UnsubscribeFeedRequested as EventPayload>::EVENT_TYPE => {
                decode_payload::<UnsubscribeFeedRequested>(payload_json)
                    .map(EventPayload::into_event)
            }
            <UnsubscribeFeedRejected as EventPayload>::EVENT_TYPE => {
                decode_payload::<UnsubscribeFeedRejected>(payload_json)
                    .map(EventPayload::into_event)
            }
            <FeedSubscribed as EventPayload>::EVENT_TYPE => {
                decode_payload::<FeedSubscribed>(payload_json).map(EventPayload::into_event)
            }
            <SubscriptionChanged as EventPayload>::EVENT_TYPE => {
                decode_payload::<SubscriptionChanged>(payload_json).map(EventPayload::into_event)
            }
            <FeedUnsubscribed as EventPayload>::EVENT_TYPE => {
                decode_payload::<FeedUnsubscribed>(payload_json).map(EventPayload::into_event)
            }
            <ApiFeedSubscribed as EventPayload>::EVENT_TYPE => {
                decode_payload::<ApiFeedSubscribed>(payload_json).map(EventPayload::into_event)
            }
            <ApiFeedSubscribeRejected as EventPayload>::EVENT_TYPE => {
                decode_payload::<ApiFeedSubscribeRejected>(payload_json)
                    .map(EventPayload::into_event)
            }
            <ApiFeedSubscriptionChanged as EventPayload>::EVENT_TYPE => {
                decode_payload::<ApiFeedSubscriptionChanged>(payload_json)
                    .map(EventPayload::into_event)
            }
            <ApiFeedUnsubscribed as EventPayload>::EVENT_TYPE => {
                decode_payload::<ApiFeedUnsubscribed>(payload_json).map(EventPayload::into_event)
            }
            <ApiFeedUnsubscribeRejected as EventPayload>::EVENT_TYPE => {
                decode_payload::<ApiFeedUnsubscribeRejected>(payload_json)
                    .map(EventPayload::into_event)
            }
            event_type => Err(EventEncodingError::UnknownEventType(event_type.to_owned())),
        }
    }
}

impl EventKind {
    pub const fn event_type(self) -> &'static str {
        match self {
            Self::Request(kind) => kind.event_type(),
            Self::Sub(kind) => kind.event_type(),
            Self::Api(kind) => kind.event_type(),
        }
    }
}

fn encode_payload<T>(payload: &T) -> EventEncodingResult<EncodedEvent>
where
    T: EventPayload,
{
    Ok(EncodedEvent {
        event_type: T::EVENT_TYPE,
        payload_json: serde_json::to_string(payload)?,
    })
}

fn decode_payload<T>(payload_json: &str) -> EventEncodingResult<T>
where
    T: EventPayload,
{
    serde_json::from_str(payload_json).map_err(EventEncodingError::from)
}
