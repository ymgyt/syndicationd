use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;
use uuid::Uuid;

use crate::subscription::SubscriberId;

mod handler;
mod projection;
pub mod query;

pub(crate) use handler::TimelineHandler;
pub use projection::{TimelineProj, TimelineProjInput};

/// Stable identity of a timeline, independent of its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(transparent)]
pub struct TimelineId(Uuid);

impl TimelineId {
    pub(crate) fn generate() -> Self {
        Self(Uuid::new_v4())
    }
}

impl fmt::Display for TimelineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for TimelineId {
    type Err = uuid::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        value.parse().map(Self)
    }
}

/// Result of catching up one feed into a subscriber's timelines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineCatchup {
    subscriber_id: SubscriberId,
    feed_url: FeedUrl,
    inserted_items: u64,
}

impl TimelineCatchup {
    pub fn new(subscriber_id: SubscriberId, feed_url: FeedUrl, inserted_items: u64) -> Self {
        Self {
            subscriber_id,
            feed_url,
            inserted_items,
        }
    }

    pub fn subscriber_id(&self) -> &SubscriberId {
        &self.subscriber_id
    }

    pub fn feed_url(&self) -> &FeedUrl {
        &self.feed_url
    }

    pub fn inserted_items(&self) -> u64 {
        self.inserted_items
    }
}
