use std::sync::Arc;

use async_trait::async_trait;

use crate::repository;

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

    async fn fetch_subscribed_feed_urls(&self, _user_id: &str) -> RepositoryResult<Vec<String>>;
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

    async fn fetch_subscribed_feed_urls(&self, user_id: &str) -> RepositoryResult<Vec<String>> {
        self.fetch_subscribed_feed_urls(user_id).await
    }
}
