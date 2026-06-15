use serde_json;
use thiserror::Error;

use crate::api::{
    ApiCrawlJobEnqueued, ApiCrawlJobFinished, ApiCrawlJobStarted, ApiEntryChanged,
    ApiEntryDiscovered, ApiFeedChanged, ApiFeedDiscovered, ApiFeedSubscribeRejected,
    ApiFeedSubscribed, ApiFeedSubscriptionChanged, ApiFeedUnsubscribeRejected, ApiFeedUnsubscribed,
    ApiTimelineChanged,
};

use super::domain::{
    CrawlJobEnqueuedEvent, CrawlJobFinishedEvent, CrawlJobStartedEvent, CrawlTargetActivatedEvent,
    CrawlTargetDeactivatedEvent, CrawlTargetPolicyChangedEvent, EntryChangedEvent,
    EntryDiscoveredEvent, Event, EventType, FeedChangedEvent, FeedDiscoveredEvent,
    FeedSubscribedEvent, FeedUnsubscribedEvent, RegistryEvent, SubscribeFeedRejected,
    SubscribeFeedRequested, SubscriptionChangedEvent, TimelineChangedEvent,
    UnsubscribeFeedRejected, UnsubscribeFeedRequested,
};

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
    pub event_type: EventType,
    pub payload_json: String,
}

pub trait EventEncoding: Sized {
    fn encode(&self) -> EventEncodingResult<EncodedEvent>;

    fn decode(event_type: &str, payload_json: &str) -> EventEncodingResult<Self>;
}

impl EventEncoding for Event {
    fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        match self {
            Self::SubscribeFeedRequested(event) => encode_payload(event),
            Self::SubscribeFeedRejected(event) => encode_payload(event),
            Self::UnsubscribeFeedRequested(event) => encode_payload(event),
            Self::UnsubscribeFeedRejected(event) => encode_payload(event),
            Self::FeedSubscribed(event) => encode_payload(event),
            Self::SubscriptionChanged(event) => encode_payload(event),
            Self::FeedUnsubscribed(event) => encode_payload(event),
            Self::CrawlTargetActivated(event) => encode_payload(event),
            Self::CrawlTargetPolicyChanged(event) => encode_payload(event),
            Self::CrawlTargetDeactivated(event) => encode_payload(event),
            Self::CrawlJobEnqueued(event) => encode_payload(event),
            Self::CrawlJobStarted(event) => encode_payload(event),
            Self::CrawlJobFinished(event) => encode_payload(event),
            Self::FeedDiscovered(event) => encode_payload(event),
            Self::FeedChanged(event) => encode_payload(event),
            Self::EntryDiscovered(event) => encode_payload(event),
            Self::EntryChanged(event) => encode_payload(event),
            Self::TimelineChanged(event) => encode_payload(event),
            Self::ApiFeedSubscribed(event) => encode_payload(event),
            Self::ApiFeedSubscribeRejected(event) => encode_payload(event),
            Self::ApiFeedSubscriptionChanged(event) => encode_payload(event),
            Self::ApiFeedUnsubscribed(event) => encode_payload(event),
            Self::ApiFeedUnsubscribeRejected(event) => encode_payload(event),
            Self::ApiCrawlJobEnqueued(event) => encode_payload(event),
            Self::ApiCrawlJobStarted(event) => encode_payload(event),
            Self::ApiCrawlJobFinished(event) => encode_payload(event),
            Self::ApiFeedDiscovered(event) => encode_payload(event),
            Self::ApiFeedChanged(event) => encode_payload(event),
            Self::ApiEntryDiscovered(event) => encode_payload(event),
            Self::ApiEntryChanged(event) => encode_payload(event),
            Self::ApiTimelineChanged(event) => encode_payload(event),
        }
    }

    fn decode(event_type: &str, payload_json: &str) -> EventEncodingResult<Self> {
        let Some(event_type) = EventType::from_wire(event_type) else {
            return Err(EventEncodingError::UnknownEventType(event_type.to_owned()));
        };

        match event_type {
            EventType::SubscribeFeedRequested => {
                decode_payload::<SubscribeFeedRequested>(payload_json).map(Event::from)
            }
            EventType::SubscribeFeedRejected => {
                decode_payload::<SubscribeFeedRejected>(payload_json).map(Event::from)
            }
            EventType::UnsubscribeFeedRequested => {
                decode_payload::<UnsubscribeFeedRequested>(payload_json).map(Event::from)
            }
            EventType::UnsubscribeFeedRejected => {
                decode_payload::<UnsubscribeFeedRejected>(payload_json).map(Event::from)
            }
            EventType::FeedSubscribed => {
                decode_payload::<FeedSubscribedEvent>(payload_json).map(Event::from)
            }
            EventType::SubscriptionChanged => {
                decode_payload::<SubscriptionChangedEvent>(payload_json).map(Event::from)
            }
            EventType::FeedUnsubscribed => {
                decode_payload::<FeedUnsubscribedEvent>(payload_json).map(Event::from)
            }
            EventType::CrawlTargetActivated => {
                decode_payload::<CrawlTargetActivatedEvent>(payload_json).map(Event::from)
            }
            EventType::CrawlTargetPolicyChanged => {
                decode_payload::<CrawlTargetPolicyChangedEvent>(payload_json).map(Event::from)
            }
            EventType::CrawlTargetDeactivated => {
                decode_payload::<CrawlTargetDeactivatedEvent>(payload_json).map(Event::from)
            }
            EventType::CrawlJobEnqueued => {
                decode_payload::<CrawlJobEnqueuedEvent>(payload_json).map(Event::from)
            }
            EventType::CrawlJobStarted => {
                decode_payload::<CrawlJobStartedEvent>(payload_json).map(Event::from)
            }
            EventType::CrawlJobFinished => {
                decode_payload::<CrawlJobFinishedEvent>(payload_json).map(Event::from)
            }
            EventType::FeedDiscovered => {
                decode_payload::<FeedDiscoveredEvent>(payload_json).map(Event::from)
            }
            EventType::FeedChanged => {
                decode_payload::<FeedChangedEvent>(payload_json).map(Event::from)
            }
            EventType::EntryDiscovered => {
                decode_payload::<EntryDiscoveredEvent>(payload_json).map(Event::from)
            }
            EventType::EntryChanged => {
                decode_payload::<EntryChangedEvent>(payload_json).map(Event::from)
            }
            EventType::TimelineChanged => {
                decode_payload::<TimelineChangedEvent>(payload_json).map(Event::from)
            }
            EventType::ApiFeedSubscribed => {
                decode_payload::<ApiFeedSubscribed>(payload_json).map(Event::from)
            }
            EventType::ApiFeedSubscribeRejected => {
                decode_payload::<ApiFeedSubscribeRejected>(payload_json).map(Event::from)
            }
            EventType::ApiFeedSubscriptionChanged => {
                decode_payload::<ApiFeedSubscriptionChanged>(payload_json).map(Event::from)
            }
            EventType::ApiFeedUnsubscribed => {
                decode_payload::<ApiFeedUnsubscribed>(payload_json).map(Event::from)
            }
            EventType::ApiFeedUnsubscribeRejected => {
                decode_payload::<ApiFeedUnsubscribeRejected>(payload_json).map(Event::from)
            }
            EventType::ApiCrawlJobEnqueued => {
                decode_payload::<ApiCrawlJobEnqueued>(payload_json).map(Event::from)
            }
            EventType::ApiCrawlJobStarted => {
                decode_payload::<ApiCrawlJobStarted>(payload_json).map(Event::from)
            }
            EventType::ApiCrawlJobFinished => {
                decode_payload::<ApiCrawlJobFinished>(payload_json).map(Event::from)
            }
            EventType::ApiFeedDiscovered => {
                decode_payload::<ApiFeedDiscovered>(payload_json).map(Event::from)
            }
            EventType::ApiFeedChanged => {
                decode_payload::<ApiFeedChanged>(payload_json).map(Event::from)
            }
            EventType::ApiEntryDiscovered => {
                decode_payload::<ApiEntryDiscovered>(payload_json).map(Event::from)
            }
            EventType::ApiEntryChanged => {
                decode_payload::<ApiEntryChanged>(payload_json).map(Event::from)
            }
            EventType::ApiTimelineChanged => {
                decode_payload::<ApiTimelineChanged>(payload_json).map(Event::from)
            }
        }
    }
}

fn encode_payload<T>(payload: &T) -> EventEncodingResult<EncodedEvent>
where
    T: RegistryEvent,
{
    Ok(EncodedEvent {
        event_type: T::TYPE,
        payload_json: serde_json::to_string(payload)?,
    })
}

fn decode_payload<T>(payload_json: &str) -> EventEncodingResult<T>
where
    T: RegistryEvent,
{
    serde_json::from_str(payload_json).map_err(EventEncodingError::from)
}
