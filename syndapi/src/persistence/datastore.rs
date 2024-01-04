use synd::Feed;

use super::DatastoreError;

pub type DatastoreResult<T> = std::result::Result<T, DatastoreError>;

pub struct Datastore {}

impl Datastore {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {})
    }

    pub async fn fetch_subscription_feeds(&self, user_id: &str) -> DatastoreResult<Vec<Feed>> {
        Ok(vec![
            synd::Feed::new(user_id.into()),
            synd::Feed::new("bar".into()),
        ])
    }
}
