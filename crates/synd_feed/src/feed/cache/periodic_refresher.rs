use std::{sync::Arc, time::Duration};

use synd_o11y::metric;
use tracing::{error, info, warn};

use crate::feed::parser::FetchFeed;

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

    fn emit_metrics(&self) {
        // Should call cache.run_pending_tasks() ?
        let entry_count = self.cache.entry_count();
        let size = self.cache.weighted_size();

        metric!(counter_cache.feed.count = entry_count);
        metric!(counter_cache.feed.size = size);
    }
}

impl<S> PeriodicRefresher<S>
where
    S: FetchFeed + Clone + 'static,
{
    #[tracing::instrument(skip_all, name = "feed::cache::refresh")]
    async fn refresh(&self) -> anyhow::Result<()> {
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
        Ok(())
    }

    pub async fn run(self, interval: Duration) {
        info!(?interval, "Run periodic feed cache refresher");

        let mut interval = tokio::time::interval(interval);

        // Consume initial tick which return ready immediately
        interval.tick().await;

        loop {
            interval.tick().await;

            if self.emit_metrics {
                self.emit_metrics();
            }
            if let Err(err) = self.refresh().await {
                error!("Periodic refresh error: {err}");
            } else {
                info!("Refreshed feed cache");
            }
        }
    }
}
