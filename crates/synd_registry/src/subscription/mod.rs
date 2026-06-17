//! Subscription identities and current relation state.

use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::crawl::policy::CrawlPolicy;

pub mod decider;
pub mod query;

pub use decider::{
    Decider, SubscriptionCommand, SubscriptionDecider, SubscriptionReject, SubscriptionState,
    decide, evolve,
};

/// Opaque registry identity that owns subscriptions.
///
/// API and UI layers decide how authenticated principals map to this value.
/// The registry does not model local users or authentication providers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriberId(String);

impl SubscriberId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SubscriberId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

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

/// Registry-owned attributes applied to one subscriber/feed relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeedSubscriptionAttrs {
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: CrawlPolicy,
}

/// Result of applying a subscribe operation to current subscription state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscribeOutcome {
    Subscribed(SubscriptionKey),
    Changed(SubscriptionKey),
}

/// Result of applying an unsubscribe operation to current subscription state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsubscribeOutcome {
    Unsubscribed(SubscriptionKey),
}

/// Current subscription attributes for one subscriber/feed relation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Subscription {
    pub subscriber_id: SubscriberId,
    pub feed_url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: CrawlPolicy,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
