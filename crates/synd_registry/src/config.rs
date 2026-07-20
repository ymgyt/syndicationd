use std::time::Duration;

use crate::crawl::{
    policy::{CrawlPolicy, PollingInterval},
    worker::CrawlWorkerPoolConfig,
};

/// Poll intervals for each registry background worker family.
#[derive(Debug, Clone, Copy)]
pub struct FeedRegistryWorkerConfig {
    pub crawl_target_projection_poll_interval: Duration,
    pub feed_projection_poll_interval: Duration,
    pub timeline_projection_poll_interval: Duration,
    pub api_event_publisher_poll_interval: Duration,
    pub crawl_dispatcher_poll_interval: Duration,
}

impl FeedRegistryWorkerConfig {
    pub fn with_poll_interval(poll_interval: Duration) -> Self {
        Self {
            crawl_target_projection_poll_interval: poll_interval,
            feed_projection_poll_interval: poll_interval,
            timeline_projection_poll_interval: poll_interval,
            api_event_publisher_poll_interval: poll_interval,
            crawl_dispatcher_poll_interval: poll_interval,
        }
    }
}

impl Default for FeedRegistryWorkerConfig {
    fn default() -> Self {
        Self::with_poll_interval(Duration::from_secs(30))
    }
}

/// Runtime configuration for the crawl dispatcher.
#[derive(Debug, Clone, Copy)]
pub struct CrawlDispatchConfig {
    /// Delay before re-checking when due feeds remain but the dispatch queue
    /// was saturated.
    pub saturated_retry_delay: Duration,
}

impl Default for CrawlDispatchConfig {
    fn default() -> Self {
        Self {
            saturated_retry_delay: Duration::from_secs(1),
        }
    }
}

/// Runtime configuration for the registry facade and event workers.
#[derive(Debug, Clone, Copy)]
pub struct FeedRegistryConfig {
    pub default_crawl_policy: CrawlPolicy,
    pub event_wake_channel_capacity: usize,
    pub workers: FeedRegistryWorkerConfig,
    pub crawl_dispatch: CrawlDispatchConfig,
    pub crawl_worker_pool: CrawlWorkerPoolConfig,
}

impl Default for FeedRegistryConfig {
    fn default() -> Self {
        Self {
            default_crawl_policy: CrawlPolicy::interval(
                PollingInterval::try_from(Duration::from_hours(2))
                    .expect("default polling interval is non-zero"),
            ),
            event_wake_channel_capacity: 1024,
            workers: FeedRegistryWorkerConfig::default(),
            crawl_dispatch: CrawlDispatchConfig::default(),
            crawl_worker_pool: CrawlWorkerPoolConfig::default(),
        }
    }
}
