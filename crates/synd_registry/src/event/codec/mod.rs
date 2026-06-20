use thiserror::Error;

use super::domain::{Event, EventType};

/// Result type returned while encoding or decoding registry events.
pub type EventEncodingResult<T> = Result<T, EventEncodingError>;

/// Error returned when converting between typed events and persisted payloads.
#[derive(Debug, Error)]
pub enum EventEncodingError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("unknown event type: {0}")]
    UnknownEventType(String),
    #[error("event type mismatch: column={column:?}, payload={payload:?}")]
    EventTypeMismatch {
        column: EventType,
        payload: EventType,
    },
}

/// Serialized representation of one registry event payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedEvent {
    pub event_type: EventType,
    pub payload_json: String,
}

/// Converts registry events to and from the journal payload shape.
pub trait EventEncoding: Sized {
    fn encode(&self) -> EventEncodingResult<EncodedEvent>;

    fn decode(event_type: &str, payload_json: &str) -> EventEncodingResult<Self>;
}

impl EventEncoding for Event {
    fn encode(&self) -> EventEncodingResult<EncodedEvent> {
        Ok(EncodedEvent {
            event_type: self.event_type(),
            payload_json: serde_json::to_string(self)?,
        })
    }

    fn decode(event_type: &str, payload_json: &str) -> EventEncodingResult<Self> {
        let column = event_type
            .parse::<EventType>()
            .map_err(|_| EventEncodingError::UnknownEventType(event_type.to_owned()))?;
        let event: Event = serde_json::from_str(payload_json)?;
        let payload = event.event_type();
        if column != payload {
            return Err(EventEncodingError::EventTypeMismatch { column, payload });
        }
        Ok(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        crawl::policy::{CrawlPolicy, PollingInterval},
        event::FeedSubscribedEvent,
        subscription::{FeedSubscriptionAttrs, SubscriberId, SubscriptionKey},
    };
    use std::time::Duration;
    use synd_feed::types::FeedUrl;

    #[test]
    fn event_encoding_roundtrips_tagged_event() {
        let event = Event::from(FeedSubscribedEvent::new(
            SubscriptionKey::new(
                SubscriberId::new("reader"),
                FeedUrl::parse("https://example.com/feed.xml").unwrap(),
            ),
            FeedSubscriptionAttrs {
                requirement: None,
                category: None,
                crawl_policy: CrawlPolicy::interval(
                    PollingInterval::try_from(Duration::from_hours(1)).unwrap(),
                ),
            },
        ));

        let encoded = event.encode().unwrap();
        assert_eq!(encoded.event_type, EventType::FeedSubscribed);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&encoded.payload_json).unwrap(),
            serde_json::json!({
                "type": "sub.feed.subscribed",
                "subscription": {
                    "subscriber_id": "reader",
                    "feed_url": "https://example.com/feed.xml"
                },
                "attrs": {
                    "requirement": null,
                    "category": null,
                    "crawl_policy": {
                        "polling": {
                            "kind": "interval",
                            "interval_seconds": 3600
                        }
                    }
                }
            })
        );
        let event_type: &'static str = encoded.event_type.into();

        assert_eq!(
            Event::decode(event_type, &encoded.payload_json).unwrap(),
            event
        );
    }
}
