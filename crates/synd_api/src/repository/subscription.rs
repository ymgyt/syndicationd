use async_trait::async_trait;

use crate::repository::{self, types::SubscribedFeeds};

use super::RepositoryError;

pub type RepositoryResult<T> = std::result::Result<T, RepositoryError>;

#[async_trait]
pub trait SubscriptionRepository: Send + Sync {
    async fn put_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()>;

    async fn delete_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()>;

    async fn fetch_subscribed_feeds(&self, _user_id: &str) -> RepositoryResult<SubscribedFeeds>;
}
