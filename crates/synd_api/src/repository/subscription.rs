use std::sync::Arc;

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

#[async_trait]
impl<T> SubscriptionRepository for Arc<T>
where
    T: SubscriptionRepository,
{
    async fn put_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        self.put_feed_subscription(feed).await
    }

    async fn delete_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        self.delete_feed_subscription(feed).await
    }

    async fn fetch_subscribed_feeds(&self, user_id: &str) -> RepositoryResult<SubscribedFeeds> {
        self.fetch_subscribed_feeds(user_id).await
    }
}
