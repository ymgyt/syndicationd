use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use synd_feed::types::{Category, FeedUrl, Requirement};

use super::RepositoryError;

pub type RepositoryResult<T> = std::result::Result<T, RepositoryError>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSubscription {
    pub user_id: String,
    pub url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SubscribedFeeds {
    pub urls: Vec<FeedUrl>,
    pub annotations: Option<HashMap<FeedUrl, FeedAnnotations>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FeedAnnotations {
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}

#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn put_feed_subscription(&self, feed: FeedSubscription) -> RepositoryResult<()>;

    async fn delete_feed_subscription(&self, feed: FeedSubscription) -> RepositoryResult<()>;

    async fn fetch_subscribed_feeds(&self, _user_id: &str) -> RepositoryResult<SubscribedFeeds>;
}
