use std::sync::RwLock;

use async_trait::async_trait;
use synd::types::FeedMeta;

use super::{datastore::DatastoreResult, Datastore};

pub struct MemoryDatastore {
    feeds: RwLock<Vec<FeedMeta>>,
}

impl MemoryDatastore {
    pub fn new() -> Self {
        Self {
            feeds: RwLock::new(vec![FeedMeta::new(
                "This week in Rust".into(),
                "https://this-week-in-rust.org/atom.xml".into(),
            )]),
        }
    }
}

#[async_trait]
impl Datastore for MemoryDatastore {
    async fn add_feed_to_subscription(
        &self,
        _user_id: &str,
        title: String,
        url: String,
    ) -> DatastoreResult<()> {
        self.feeds.write().unwrap().push(FeedMeta::new(title, url));
        Ok(())
    }

    async fn fetch_subscription_feeds(&self, _user_id: &str) -> DatastoreResult<Vec<FeedMeta>> {
        Ok(self.feeds.read().unwrap().clone())
    }
}
