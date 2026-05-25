use synd_feed::{
    feed::service::{FeedService, FetchFeedError},
    types::{Feed, FeedUrl},
};
use thiserror::Error;

use crate::model::FeedSnapshot;

#[derive(Debug, Clone)]
pub struct FetchedFeed {
    pub feed_url: FeedUrl,
    pub feed: Feed,
    pub snapshot: FeedSnapshot,
}

#[derive(Debug, Error)]
pub enum FeedProviderError {
    #[error(transparent)]
    Fetch(#[from] FetchFeedError),
}

pub trait FeedProvider: Clone + Send + Sync + 'static {
    async fn fetch(&self, feed_url: FeedUrl) -> Result<FetchedFeed, FeedProviderError>;
    fn parse_snapshot(&self, snapshot: &FeedSnapshot) -> Result<Feed, FeedProviderError>;
}

#[derive(Clone)]
pub struct SyndFeedProvider {
    service: FeedService,
}

impl SyndFeedProvider {
    pub fn new(service: FeedService) -> Self {
        Self { service }
    }
}

impl FeedProvider for SyndFeedProvider {
    async fn fetch(&self, feed_url: FeedUrl) -> Result<FetchedFeed, FeedProviderError> {
        let fetched = self.service.fetch_feed_with_body(feed_url).await?;
        let snapshot = FeedSnapshot {
            feed_url: fetched.url.clone(),
            body: fetched.body,
            content_type: fetched.content_type,
            etag: fetched.etag,
            last_modified: fetched.last_modified,
            fetched_at: fetched.fetched_at,
        };

        Ok(FetchedFeed {
            feed_url: fetched.url,
            feed: fetched.feed,
            snapshot,
        })
    }

    fn parse_snapshot(&self, snapshot: &FeedSnapshot) -> Result<Feed, FeedProviderError> {
        self.service
            .parse(snapshot.feed_url.clone(), snapshot.body.as_slice())
            .map_err(FeedProviderError::Fetch)
    }
}
