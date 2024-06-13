use std::time::Duration;

use crate::config;

#[derive(Debug, Clone)]
pub struct Config {
    pub idle_timer_interval: Duration,
    pub throbber_timer_interval: Duration,
    pub entries_limit: usize,
    pub entries_per_pagination: i64,
    pub feeds_per_pagination: i64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle_timer_interval: Duration::from_secs(250),
            throbber_timer_interval: Duration::from_millis(250),
            entries_limit: config::feed::DEFAULT_ENTRIES_LIMIT,
            entries_per_pagination: config::client::DEFAULT_ENTRIES_PER_PAGINATION,
            feeds_per_pagination: config::client::DEFAULT_FEEDS_PER_PAGINATION,
        }
    }
}

impl Config {
    #[must_use]
    pub fn with_idle_timer_interval(self, idle_timer_interval: Duration) -> Self {
        Self {
            idle_timer_interval,
            ..self
        }
    }
}
