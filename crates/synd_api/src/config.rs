pub const USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub mod app {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const NAME: &str = env!("CARGO_PKG_NAME");
}

pub mod serve {
    pub const DEFAULT_REQUEST_BODY_LIMIT_BYTES: usize = 1024 * 2;
    pub const DEFAULT_REQUEST_CONCURRENCY_LIMIT: usize = 100;

    pub const HEALTH_CHECK_PATH: &str = "/health";
}

pub mod metrics {
    use std::time::Duration;

    pub const MONITOR_INTERVAL: Duration = Duration::from_mins(1);
}
