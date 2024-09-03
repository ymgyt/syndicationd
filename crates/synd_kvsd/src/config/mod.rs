/// Application configurations
pub mod app {
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    pub const NAME: &str = env!("CARGO_PKG_NAME");
}

/// Environment variable configurations
pub mod env {
    macro_rules! env_key {
        ($key:expr) => {
            concat!("KVSD", "_", $key)
        };
    }
    /// log directive for tracing subscriber env filter
    pub const LOG_DIRECTIVE: &str = env_key!("LOG");
    pub const LOG_SHOW_LOCATION: &str = env_key!("LOG_SHOW_LOCATION");
    pub const LOG_SHOW_TARGET: &str = env_key!("LOG_SHOW_TARGET");

    pub const MAX_CONNECTIONS: &str = env_key!("MAX_CONNECTIONS");
    pub const CONNECTION_BUFFER_BYTES: &str = env_key!("CONNECTION_BUFFER_BYTES");
    pub const AUTHENTICATE_TIMEOUT: &str = env_key!("AUTHENTICATE_TIMEOUT");
    pub const CONFIG_FILE: &str = env_key!("CONFIG_FILE");

    pub const BIND_ADDRESS: &str = env_key!("BIND_ADDRESS");
    pub const BIND_PORT: &str = env_key!("BIND_PORT");
    pub const DATA_DIR: &str = env_key!("DATA_DIR");

    pub const TLS_CERT: &str = env_key!("TLS_CERT");
    pub const TLS_KEY: &str = env_key!("TLS_KEY");
}
