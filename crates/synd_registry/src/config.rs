use std::time::Duration;

use crate::crawl::policy::RefreshInterval;

#[derive(Debug, Clone, Copy)]
pub struct FeedRegistryConfig {
    pub default_refresh_interval: RefreshInterval,
    pub event_wake_channel_capacity: usize,
    pub event_worker_poll_interval: Duration,
}

impl Default for FeedRegistryConfig {
    fn default() -> Self {
        Self {
            default_refresh_interval: RefreshInterval::try_from(Duration::from_hours(2))
                .expect("default refresh interval is non-zero"),
            event_wake_channel_capacity: 1024,
            event_worker_poll_interval: Duration::from_secs(30),
        }
    }
}
