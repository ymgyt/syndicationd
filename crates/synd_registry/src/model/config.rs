use std::time::Duration;

use super::RefreshInterval;

#[derive(Debug, Clone, Copy)]
pub struct FeedRegistryConfig {
    pub default_refresh_interval: RefreshInterval,
    pub scheduler_tick_interval: Duration,
    pub refresh_concurrency: usize,
    pub refresh_lease_duration: Duration,
    pub store_retry_delay: Duration,
}

impl Default for FeedRegistryConfig {
    fn default() -> Self {
        Self {
            default_refresh_interval: RefreshInterval::try_from(Duration::from_hours(2))
                .expect("default refresh interval is non-zero"),
            scheduler_tick_interval: Duration::from_mins(5),
            refresh_concurrency: 10,
            refresh_lease_duration: Duration::from_mins(10),
            store_retry_delay: Duration::from_secs(30),
        }
    }
}
