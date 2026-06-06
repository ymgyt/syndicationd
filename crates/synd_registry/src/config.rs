use std::time::Duration;

use crate::crawl::{
    policy::{CrawlPolicy, PollingInterval},
    worker::CrawlWorkerPoolConfig,
};

#[derive(Debug, Clone, Copy)]
pub struct FeedRegistryConfig {
    pub default_crawl_policy: CrawlPolicy,
    pub event_wake_channel_capacity: usize,
    pub workers: FeedRegistryWorkerConfig,
    pub crawl_worker_pool: CrawlWorkerPoolConfig,
}

#[derive(Debug, Clone, Copy)]
pub struct FeedRegistryWorkerConfig {
    pub subscription_request_poll_interval: Duration,
    pub crawl_target_projection_poll_interval: Duration,
    pub api_event_projection_poll_interval: Duration,
    pub api_event_publisher_poll_interval: Duration,
    pub crawl_scheduler_poll_interval: Duration,
    pub crawl_worker_pool_poll_interval: Duration,
}

impl FeedRegistryWorkerConfig {
    pub fn with_poll_interval(poll_interval: Duration) -> Self {
        Self {
            subscription_request_poll_interval: poll_interval,
            crawl_target_projection_poll_interval: poll_interval,
            api_event_projection_poll_interval: poll_interval,
            api_event_publisher_poll_interval: poll_interval,
            crawl_scheduler_poll_interval: poll_interval,
            crawl_worker_pool_poll_interval: poll_interval,
        }
    }
}

impl Default for FeedRegistryWorkerConfig {
    fn default() -> Self {
        Self::with_poll_interval(Duration::from_secs(30))
    }
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
            crawl_worker_pool: CrawlWorkerPoolConfig::default(),
        }
    }
}
