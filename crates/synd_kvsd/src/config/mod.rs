/// Application configurations
pub mod app {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const NAME: &str = env!("CARGO_PKG_NAME");
}

/// Environment variable configurations
pub mod env {
    macro_rules! env_key {
        ($key:expr) => {
            concat!("SYND", "_", $key)
        };
    }
    /// log directive for tracing subscriber env filter
    pub const LOG_DIRECTIVE: &str = env_key!("LOG");
    pub const LOG_SHOW_LOCATION: &str = env_key!("LOG_SHOW_LOCATION");
    pub const LOG_SHOW_TARGET: &str = env_key!("LOG_SHOW_TARGET");
}
