use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use moka::future::Cache;

use crate::{
    feed::parser::{FetchFeed, ParseResult},
    types,
};

pub struct CacheConfig {
    max_cache_size: u64,
    time_to_live: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            // 10MiB
            max_cache_size: 10 * 1024 * 1024,
            time_to_live: Duration::from_secs(60 * 60),
        }
    }
}

impl CacheConfig {
    pub fn with_max_cache_size(self, max_cache_size: u64) -> Self {
        Self {
            max_cache_size,
            ..self
        }
    }

    pub fn with_time_to_live(self, time_to_live: Duration) -> Self {
        Self {
            time_to_live,
            ..self
        }
    }
}

#[async_trait]
pub trait FetchCachedFeed: Send + Sync {
    async fn fetch_feed(&self, url: String) -> ParseResult<Arc<types::Feed>>;
    /// Fetch feeds by spawing tasks
    async fn fetch_feeds_parallel(&self, urls: &[String]) -> Vec<ParseResult<Arc<types::Feed>>>;
}

#[derive(Clone)]
pub struct CacheLayer<S> {
    service: S,
    // Use Arc to avoid expensive clone
    // https://github.com/moka-rs/moka?tab=readme-ov-file#avoiding-to-clone-the-value-at-get
    cache: Cache<String, Arc<types::Feed>>,
}
impl<S> CacheLayer<S> {
    /// Construct CacheLayer with default config
    pub fn new(service: S) -> Self {
        Self::with(service, CacheConfig::default())
    }

    /// Construct CacheLayer with given config
    pub fn with(service: S, config: CacheConfig) -> Self {
        let CacheConfig {
            max_cache_size,
            time_to_live,
        } = config;

        let cache = Cache::builder()
            .weigher(|_key, value: &Arc<types::Feed>| -> u32 {
                value.approximate_size().try_into().unwrap_or(u32::MAX)
            })
            .max_capacity(max_cache_size)
            .time_to_live(time_to_live)
            .build();

        Self { service, cache }
    }
}

#[async_trait]
impl<S> FetchCachedFeed for CacheLayer<S>
where
    S: FetchFeed + Clone + 'static,
{
    async fn fetch_feed(&self, url: String) -> ParseResult<Arc<types::Feed>> {
        // lookup cache
        if let Some(feed) = self.cache.get(&url).await {
            tracing::debug!(url, "Feed cache hit");
            return Ok(feed);
        }

        let feed = self.service.fetch_feed(url.clone()).await?;
        let feed = Arc::new(feed);

        self.cache.insert(url, Arc::clone(&feed)).await;

        Ok(feed)
    }

    /// Fetch feeds by spawing tasks
    async fn fetch_feeds_parallel(&self, urls: &[String]) -> Vec<ParseResult<Arc<types::Feed>>> {
        let mut handles = Vec::with_capacity(urls.len());

        for url in urls {
            let this = self.clone();
            let url = url.clone();
            handles.push(tokio::spawn(async move { this.fetch_feed(url).await }));
        }

        let mut results = Vec::with_capacity(handles.len());

        for handle in handles {
            // panic on join error
            results.push(handle.await.unwrap());
        }

        results
    }
}
