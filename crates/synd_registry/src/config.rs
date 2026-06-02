use std::time::Duration;

use crate::crawl::policy::{CrawlPolicy, PollingInterval};

#[derive(Debug, Clone, Copy)]
pub struct FeedRegistryConfig {
    pub default_crawl_policy: CrawlPolicy,
    pub event_wake_channel_capacity: usize,
    pub event_worker_poll_interval: Duration,
}

impl Default for FeedRegistryConfig {
    fn default() -> Self {
        Self {
            default_crawl_policy: CrawlPolicy::interval(
                PollingInterval::try_from(Duration::from_hours(2))
                    .expect("default polling interval is non-zero"),
            ),
            event_wake_channel_capacity: 1024,
            event_worker_poll_interval: Duration::from_secs(30),
        }
    }
}
