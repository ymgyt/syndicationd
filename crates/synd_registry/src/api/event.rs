use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

use crate::{
    event::{EventType, RegistryEvent, RequestId},
    subscription::SubscriptionKey,
    timeline::TimelineKey,
};

/// Public event contract exposed through the API stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiEvent {
    FeedSubscribed(ApiFeedSubscribed),
    FeedSubscribeRejected(ApiFeedSubscribeRejected),
    FeedSubscriptionChanged(ApiFeedSubscriptionChanged),
    FeedUnsubscribed(ApiFeedUnsubscribed),
    FeedUnsubscribeRejected(ApiFeedUnsubscribeRejected),
    TimelineChanged(ApiTimelineChanged),
}

/// API stream payload emitted when a feed subscription is accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedSubscribed {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl ApiFeedSubscribed {
    pub fn new(request_id: RequestId, subscription: SubscriptionKey) -> Self {
        Self {
            request_id,
            subscription,
        }
    }
}

/// API stream payload emitted when a feed subscribe request is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedSubscribeRejected {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
    pub reason: String,
}

impl ApiFeedSubscribeRejected {
    pub fn new(
        request_id: RequestId,
        subscription: SubscriptionKey,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            subscription,
            reason: reason.into(),
        }
    }
}

/// API stream payload emitted when an existing feed subscription changes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedSubscriptionChanged {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl ApiFeedSubscriptionChanged {
    pub fn new(request_id: RequestId, subscription: SubscriptionKey) -> Self {
        Self {
            request_id,
            subscription,
        }
    }
}

/// API stream payload emitted when a feed subscription is removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedUnsubscribed {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
}

impl ApiFeedUnsubscribed {
    pub fn new(request_id: RequestId, subscription: SubscriptionKey) -> Self {
        Self {
            request_id,
            subscription,
        }
    }
}

/// API stream payload emitted when a feed unsubscribe request is rejected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiFeedUnsubscribeRejected {
    pub request_id: RequestId,
    pub subscription: SubscriptionKey,
    pub reason: String,
}

impl ApiFeedUnsubscribeRejected {
    pub fn new(
        request_id: RequestId,
        subscription: SubscriptionKey,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            request_id,
            subscription,
            reason: reason.into(),
        }
    }
}

/// API stream payload emitted when a timeline's visible contents change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiTimelineChanged {
    pub timeline: TimelineKey,
    pub changed_at: DateTime<Utc>,
    pub affected_feeds: Vec<FeedUrl>,
}

impl ApiTimelineChanged {
    pub fn new(
        timeline: TimelineKey,
        changed_at: DateTime<Utc>,
        affected_feeds: Vec<FeedUrl>,
    ) -> Self {
        Self {
            timeline,
            changed_at,
            affected_feeds,
        }
    }
}

impl RegistryEvent for ApiFeedSubscribed {
    const TYPE: EventType = EventType::ApiFeedSubscribed;
}

impl RegistryEvent for ApiFeedSubscribeRejected {
    const TYPE: EventType = EventType::ApiFeedSubscribeRejected;
}

impl RegistryEvent for ApiFeedSubscriptionChanged {
    const TYPE: EventType = EventType::ApiFeedSubscriptionChanged;
}

impl RegistryEvent for ApiFeedUnsubscribed {
    const TYPE: EventType = EventType::ApiFeedUnsubscribed;
}

impl RegistryEvent for ApiFeedUnsubscribeRejected {
    const TYPE: EventType = EventType::ApiFeedUnsubscribeRejected;
}

impl RegistryEvent for ApiTimelineChanged {
    const TYPE: EventType = EventType::ApiTimelineChanged;
}
