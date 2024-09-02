pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub mod app {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const NAME: &str = env!("CARGO_PKG_NAME");
}

pub const PORT: u16 = 5959;

pub mod env {
    macro_rules! env_key {
        ($key:expr) => {
            concat!("SYND", "_", $key)
        };
    }
    pub(crate) use env_key;
}

pub mod serve {
    pub const DEFAULT_ADDR: &str = "127.0.0.1";
    pub const DEFAULT_PORT: u16 = 5959;
    pub const DEFAULT_REQUEST_TIMEOUT: &str = "30s";
    pub const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 1024 * 2;
    pub const DEFAULT_REQUEST_CONCURRENCY_LIMIT: usize = 100;

    pub const HEALTH_CHECK_PATH: &str = "/health";
}

pub mod metrics {
    use std::time::Duration;

    pub const MONITOR_INTERVAL: Duration = Duration::from_secs(60);
}

pub mod cache {
    pub const DEFAULT_FEED_CACHE_SIZE_MB: u64 = 100;
    pub const DEFAULT_FEED_CACHE_TTL: &str = "180min";
    pub const DEFAULT_FEED_CACHE_REFRESH_INTERVAL: &str = "120min";
}
