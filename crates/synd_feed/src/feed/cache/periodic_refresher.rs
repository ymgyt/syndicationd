use std::{sync::Arc, time::Duration};

use synd_o11y::metric;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use crate::feed::service::FetchFeed;

use super::Cache;

pub struct PeriodicRefresher<S> {
    service: S,
    cache: Cache,
    emit_metrics: bool,
}

impl<S> PeriodicRefresher<S> {
    pub fn new(service: S, cache: Cache) -> Self {
        Self {
            service,
            cache,
            emit_metrics: false,
        }
    }

    #[must_use]
    pub fn with_emit_metrics(self, emit_metrics: bool) -> Self {
        Self {
            emit_metrics,
            ..self
        }
    }

    fn emit_metrics(&self, prev: &Metrics) -> Metrics {
        // Should call cache.run_pending_tasks() ?
        let current = Metrics {
            cache_count: self.cache.entry_count().try_into().unwrap_or(0),
            cache_size: self.cache.weighted_size().try_into().unwrap_or(0),
        };

        metric!(counter.cache.feed.count = current.cache_count - prev.cache_count);
        metric!(counter.cache.feed.size = current.cache_size - prev.cache_size);

        current
    }
}

impl<S> PeriodicRefresher<S>
where
    S: FetchFeed + Clone + 'static,
{
    #[tracing::instrument(skip_all, name = "feed::cache::refresh")]
    async fn refresh(&self) {
        // It is safe to insert while iterating to cache.
        for (feed_url, _) in &self.cache {
            let feed_url = Arc::unwrap_or_clone(feed_url);
            match self.service.fetch_feed(feed_url.clone()).await {
                Ok(new_feed) => {
                    self.cache.insert(feed_url, Arc::new(new_feed)).await;
                }
                Err(err) => {
                    warn!(
                        url = feed_url.as_str(),
                        "Failed to refresh feed cache: {err}"
                    );
                }
            }
        }
    }

    pub async fn run(self, interval: Duration, ct: CancellationToken) {
        info!(?interval, "Run periodic feed cache refresher");

        let mut interval = tokio::time::interval(interval);
        let mut prev = Metrics::default();

        // Consume initial tick which return ready immediately
        interval.tick().await;

        loop {
            tokio::select! {
                biased;
                _ = interval.tick() => {},
                () = ct.cancelled() => break,
            }

            if self.emit_metrics {
                prev = self.emit_metrics(&prev);
            }
            self.refresh().await;
            info!("Refreshed feed cache");
        }
    }
}

#[derive(Default)]
struct Metrics {
    cache_count: i64,
    cache_size: i64,
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use url::Url;

    use crate::{
        feed::service::FetchFeedResult,
        types::{Feed, FeedUrl},
    };

    use super::*;

    #[derive(Clone)]
    struct Fetcher {}

    #[async_trait]
    impl FetchFeed for Fetcher {
        async fn fetch_feed(&self, url: FeedUrl) -> FetchFeedResult<Feed> {
            if url.as_str().ends_with("bad") {
                Err(crate::feed::service::FetchFeedError::Other(
                    anyhow::anyhow!("error"),
                ))
            } else {
                let (_, feed) = feed();
                Ok(feed)
            }
        }
    }

    #[tokio::test]
    async fn refresher() {
        let cache = {
            let cache = Cache::new(1024);
            let (url, feed) = feed();
            cache.insert(url.clone(), Arc::new(feed.clone())).await;

            let url2: Url = url.into();
            let url2 = url2.join("bad").unwrap();
            let url2: FeedUrl = url2.into();
            cache.insert(url2, Arc::new(feed)).await;
            cache
        };

        let fetcher = Fetcher {};
        let refresher = PeriodicRefresher::new(fetcher, cache).with_emit_metrics(true);
        let ct = CancellationToken::new();
        ct.cancel();

        refresher.run(Duration::from_nanos(1), ct).await;
    }

    fn feed() -> (FeedUrl, Feed) {
        let url: FeedUrl = Url::parse("https://example.ymgyt.io/atom.xml")
            .unwrap()
            .into();
        let feed = feed_rs::model::Feed {
            feed_type: feed_rs::model::FeedType::RSS1,
            id: "ID".into(),
            title: None,
            updated: None,
            authors: Vec::new(),
            description: None,
            links: Vec::new(),
            categories: Vec::new(),
            contributors: Vec::new(),
            generator: None,
            icon: None,
            language: None,
            logo: None,
            published: None,
            rating: None,
            rights: None,
            ttl: None,
            entries: Vec::new(),
        };
        let feed = (url.clone(), feed).into();
        (url, feed)
    }
}
