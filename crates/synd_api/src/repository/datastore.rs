use std::sync::Arc;

use async_trait::async_trait;

use crate::repository;

use super::DatastoreError;

pub type DatastoreResult<T> = std::result::Result<T, DatastoreError>;

#[async_trait]
pub trait Datastore: Send + Sync {
    async fn put_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> DatastoreResult<()>;

    async fn delete_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> DatastoreResult<()>;

    async fn fetch_subscribed_feed_urls(&self, _user_id: &str) -> DatastoreResult<Vec<String>>;
}

#[async_trait]
impl<T> Datastore for Arc<T>
where
    T: Datastore,
{
    async fn put_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> DatastoreResult<()> {
        self.put_feed_subscription(feed).await
    }

    async fn delete_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> DatastoreResult<()> {
        self.delete_feed_subscription(feed).await
    }

    async fn fetch_subscribed_feed_urls(&self, user_id: &str) -> DatastoreResult<Vec<String>> {
        self.fetch_subscribed_feed_urls(user_id).await
    }
}
