use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::{crawl::policy::RefreshPolicy, subscriber::SubscriberId};

/// Stable identity of one subscriber/feed relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionKey {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
}

impl SubscriptionKey {
    pub fn new(subscriber_id: SubscriberId, feed_url: FeedUrl) -> Self {
        Self {
            subscriber_id,
            feed_url,
        }
    }
}

/// Current subscription attributes for one subscriber/feed relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub refresh_policy: RefreshPolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
