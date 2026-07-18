use std::fmt;

use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

use crate::subscription::SubscriberId;

mod projection;
pub mod query;

pub use projection::{TimelineProj, TimelineProjInput};

/// Timeline definition currently supported by the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimelineKind {
    Default,
}

impl TimelineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
        }
    }
}

impl fmt::Display for TimelineKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Stable identity of one subscriber-scoped timeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimelineKey {
    pub subscriber_id: SubscriberId,
    pub kind: TimelineKind,
}

impl TimelineKey {
    pub fn default_for(subscriber_id: SubscriberId) -> Self {
        Self {
            subscriber_id,
            kind: TimelineKind::Default,
        }
    }
}

/// Result of catching up one feed into one timeline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineCatchup {
    timeline: TimelineKey,
    feed_url: FeedUrl,
    inserted_items: u64,
}

impl TimelineCatchup {
    pub fn new(timeline: TimelineKey, feed_url: FeedUrl, inserted_items: u64) -> Self {
        Self {
            timeline,
            feed_url,
            inserted_items,
        }
    }

    pub fn timeline(&self) -> &TimelineKey {
        &self.timeline
    }

    pub fn feed_url(&self) -> &FeedUrl {
        &self.feed_url
    }

    pub fn inserted_items(&self) -> u64 {
        self.inserted_items
    }
}
