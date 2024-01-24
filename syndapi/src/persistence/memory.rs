use std::sync::RwLock;

use async_trait::async_trait;

use crate::persistence::{
    self,
    datastore::{Datastore, DatastoreResult},
};

pub struct MemoryDatastore {
    feeds: RwLock<Vec<persistence::types::FeedSubscription>>,
}

impl MemoryDatastore {
    pub fn new() -> Self {
        Self {
            feeds: RwLock::new(vec![
                persistence::types::FeedSubscription {
                    user_id: "me".into(),
                    url: "https://blog.ymgyt.io/atom.xml".into(),
                },
                persistence::types::FeedSubscription {
                    user_id: "me".into(),
                    url: "https://this-week-in-rust.org/atom.xml".into(),
                },
                persistence::types::FeedSubscription {
                    user_id: "me".into(),
                    url: "https://buttondown.email/o11y.news/rss".into(),
                },
            ]),
        }
    }
}

#[async_trait]
impl Datastore for MemoryDatastore {
    async fn put_feed_subscription(
        &self,
        feed: persistence::types::FeedSubscription,
    ) -> DatastoreResult<()> {
        self.feeds.write().unwrap().push(feed);
        Ok(())
    }

    async fn delete_feed_subscription(
        &self,
        feed: persistence::types::FeedSubscription,
    ) -> DatastoreResult<()> {
        let to_delete = feed.url;
        self.feeds
            .write()
            .unwrap()
            .retain(|sub| sub.url != to_delete);
        Ok(())
    }

    async fn fetch_subscribed_feed_urls(&self, _user_id: &str) -> DatastoreResult<Vec<String>> {
        Ok(self
            .feeds
            .read()
            .unwrap()
            .iter()
            .map(|feed| feed.url.clone())
            .collect())
    }
}
