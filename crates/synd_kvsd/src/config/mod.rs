use std::{net::IpAddr, path::PathBuf, time::Duration};

use serde::Deserialize;

mod file;
mod resolver;
pub use resolver::{ConfigResolver, ConfigResolverError};
use synd_stdx::conf::Entry;

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

    pub const CONNECTIONS_LIMIT: &str = env_key!("CONNECTIONS_LIMIT");
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

mod kvsd {
    pub(super) mod default {
        use std::{net::IpAddr, time::Duration};

        use crate::config::TlsConnection;

        pub(crate) const CONNECTIONS_LIMIT: u32 = 1024;
        pub(crate) const BUFFER_SIZE_PER_CONNECTION: usize = 1024 * 1024 * 1024;
        pub(crate) const AUTHENTICATE_TIMEOUT: Duration = Duration::from_secs(3);
        pub(crate) const BIND_PORT: u16 = 7379;
        pub(crate) const TLS_CONNECTION: TlsConnection = TlsConnection::Disable;

        pub(crate) fn bind_address() -> IpAddr {
            IpAddr::from([127, 0, 0, 1])
        }
    }
}

// Server configuration.
#[derive(Debug)]
#[expect(dead_code)]
pub struct Config {
    /// Max tcp connections.
    pub(super) connections_limit: Entry<u32>,
    /// Size of buffer allocated per tcp connection.
    pub(super) buffer_size_per_connection: Entry<usize>,
    /// Timeout duration for reading authenticate message.
    pub(super) authenticate_timeout: Entry<Duration>,
    /// Bind address
    pub(super) bind_address: Entry<IpAddr>,
    /// tcp listen port.
    pub(super) bind_port: Entry<u16>,
    /// Tls connection
    pub(super) tls: Entry<TlsConnection>,
}

#[derive(Debug)]
pub enum TlsConnection {
    Enable(TlsConfig),
    Disable,
}

#[derive(Debug, Deserialize)]
#[expect(dead_code)]
pub struct TlsConfig {
    // tls server certificate file path
    tls_certificate: PathBuf,
    // tls server private key file path
    tls_key: PathBuf,
}
