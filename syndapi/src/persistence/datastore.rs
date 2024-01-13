use async_trait::async_trait;

use synd::types::FeedMeta;

use super::DatastoreError;

pub type DatastoreResult<T> = std::result::Result<T, DatastoreError>;

#[async_trait]
pub trait Datastore: Send + Sync {
    async fn add_feed_to_subscription(
        &self,
        _user_id: &str,
        title: String,
        url: String,
    ) -> DatastoreResult<()>;

    async fn fetch_subscription_feeds(&self, _user_id: &str) -> DatastoreResult<Vec<FeedMeta>>;
}
