use std::sync::RwLock;

use synd::Feed;

use super::{kvsd::KvsdClient, DatastoreError};

pub type DatastoreResult<T> = std::result::Result<T, DatastoreError>;

pub struct Datastore {
    #[allow(dead_code)]
    // use dummy for dev
    kvsd: Option<KvsdClient>,
    // tmp
    feeds: RwLock<Vec<Feed>>,
}

impl Datastore {
    pub fn new(kvsd: Option<KvsdClient>) -> anyhow::Result<Self> {
        Ok(Self {
            kvsd,
            feeds: RwLock::new(vec![Feed::new(
                "https://this-week-in-rust.org/atom.xml".into(),
            )]),
        })
    }

    pub async fn add_feed_to_subscription(
        &self,
        _user_id: &str,
        url: String,
    ) -> DatastoreResult<()> {
        self.feeds.write().unwrap().push(Feed::new(url));
        Ok(())
    }

    pub async fn fetch_subscription_feeds(&self, _user_id: &str) -> DatastoreResult<Vec<Feed>> {
        Ok(self.feeds.read().unwrap().clone())
    }
}
