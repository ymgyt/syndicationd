use synd_feed::types::FeedUrl;

use crate::subscription::SubscriberId;

mod projection;
pub mod query;

pub use projection::{TimelineProj, TimelineProjInput};

/// Result of catching up one feed into one subscriber's timeline.
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
