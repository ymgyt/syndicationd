use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

use crate::subscriber::SubscriberId;

/// A registry subscription relation identified by subscriber and feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    /// The registry subscriber that owns the subscription relation.
    pub subscriber_id: SubscriberId,
    /// The feed URL in the subscription relation.
    pub feed_url: FeedUrl,
}

impl Subscription {
    pub fn new(subscriber_id: SubscriberId, feed_url: FeedUrl) -> Self {
        Self {
            subscriber_id,
            feed_url,
        }
    }
}
