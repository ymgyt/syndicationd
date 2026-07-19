use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

use crate::subscription::SubscriberId;

/// Public event contract exposed through the API stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiEvent {
    TimelineChanged(ApiTimelineChanged),
}

impl ApiEvent {
    pub fn subscriber_id(&self) -> &SubscriberId {
        match self {
            Self::TimelineChanged(event) => &event.subscriber_id,
        }
    }
}

/// API stream payload emitted when a timeline's visible contents change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiTimelineChanged {
    pub subscriber_id: SubscriberId,
    pub changed_at: DateTime<Utc>,
    pub affected_feeds: Vec<FeedUrl>,
}

impl ApiTimelineChanged {
    pub fn new(
        subscriber_id: SubscriberId,
        changed_at: DateTime<Utc>,
        affected_feeds: Vec<FeedUrl>,
    ) -> Self {
        Self {
            subscriber_id,
            changed_at,
            affected_feeds,
        }
    }
}
