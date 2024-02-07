use std::sync::RwLock;

use async_trait::async_trait;

use crate::repository::{
    self,
    subscription::{RepositoryResult, SubscriptionRepository},
};

pub struct MemoryRepository {
    feeds: RwLock<Vec<repository::types::FeedSubscription>>,
}

const TEST_DATA: &[&str] = &[
    "https://github.com/casey/just/releases.atom",
    "https://seanmonstar.com/rss",
    "https://thesquareplanet.com/feed.xml",
    "https://thiscute.world/en/index.xml",
    "https://blog.m-ou.se/index.xml",
    "https://keens.github.io/index.xml",
    "https://without.boats/index.xml",
    "https://blog.rust-lang.org/feed.xml",
    "https://blog.ymgyt.io/atom.xml",
    "https://this-week-in-rust.org/atom.xml",
    "https://blog.orhun.dev/rss.xml",
    "https://buttondown.email/o11y.news/rss",
    "https://fasterthanli.me/index.xml",
    "https://docs.aws.amazon.com/eks/latest/userguide/doc-history.rss",
    "https://kubernetes.io/feed.xml",
    "https://blog.guillaume-gomez.fr/atom",
    "https://sgued.fr/blog/atom.xml",
    "https://thiscute.world/en/index.xml",
    "https://blog-dry.com/feed",
];

impl MemoryRepository {
    #[must_use]
    pub fn new() -> Self {
        Self {
            feeds: RwLock::new(
                TEST_DATA
                    .iter()
                    .map(|feed| repository::types::FeedSubscription {
                        user_id: "me".into(),
                        url: (*feed).to_string(),
                    })
                    .collect(),
            ),
        }
    }
}

#[async_trait]
impl SubscriptionRepository for MemoryRepository {
    async fn put_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        self.feeds.write().unwrap().push(feed);
        Ok(())
    }

    async fn delete_feed_subscription(
        &self,
        feed: repository::types::FeedSubscription,
    ) -> RepositoryResult<()> {
        let to_delete = feed.url;
        self.feeds
            .write()
            .unwrap()
            .retain(|sub| sub.url != to_delete);
        Ok(())
    }

    async fn fetch_subscribed_feed_urls(&self, _user_id: &str) -> RepositoryResult<Vec<String>> {
        Ok(self
            .feeds
            .read()
            .unwrap()
            .iter()
            .map(|feed| feed.url.clone())
            .collect())
    }
}
