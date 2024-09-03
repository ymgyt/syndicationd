use std::{path::PathBuf, time::Duration};

use serde::Deserialize;

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
    pub const DISABLE_TLS: &str = env_key!("DISABLE_TLS");
}

// Server configuration.
#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct Config {
    /// Max tcp connections.
    max_tcp_connections: Option<u32>,
    /// Size of buffer allocated per tcp connection.
    buffer_size_per_connection: Option<usize>,
    /// Timeout duration for reading authenticate message.
    authenticate_timeout: Duration,
    /// Bind address
    bind_address: Option<String>,
    // tcp listen port.
    bind_port: u16,
    // disable tls connections.
    disable_tls: bool,
    // tls server certificate file path
    tls_certificate: Option<PathBuf>,
    // tls server private key file path
    tls_key: Option<PathBuf>,
}
