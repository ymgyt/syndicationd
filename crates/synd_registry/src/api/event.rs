use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

use crate::{
    event::{EventType, RegistryEvent},
    subscription::SubscriberId,
    timeline::TimelineKey,
};

/// Public event contract exposed through the API stream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApiEvent {
    TimelineChanged(ApiTimelineChanged),
}

impl ApiEvent {
    pub fn subscriber_id(&self) -> &SubscriberId {
        match self {
            Self::TimelineChanged(event) => &event.timeline.subscriber_id,
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

impl RegistryEvent for ApiTimelineChanged {
    const TYPE: EventType = EventType::ApiTimelineChanged;
}
