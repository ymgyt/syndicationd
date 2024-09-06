use std::{net::IpAddr, path::PathBuf, time::Duration};

use serde::Deserialize;

use crate::{args::KvsdOptions, config::file::ConfigFile};

mod file;
mod resolver;
pub use resolver::{ConfigResolver, ConfigResolverError};

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

mod kvsd {
    pub(super) mod default {
        use std::{net::IpAddr, time::Duration};

        use crate::config::TlsConnection;

        pub(crate) const MAX_TCP_CONNECTIONS: u32 = 1024;
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
pub struct Config {
    /// Max tcp connections.
    connections_limit: u32,
    /// Size of buffer allocated per tcp connection.
    buffer_size_per_connection: usize,
    /// Timeout duration for reading authenticate message.
    authenticate_timeout: Duration,
    /// Bind address
    bind_address: IpAddr,
    /// tcp listen port.
    bind_port: u16,
    /// Tls connection
    tls: TlsConnection,
}

#[derive(Debug)]
pub enum TlsConnection {
    Enable(TlsConfig),
    Disable,
}

#[derive(Debug, Deserialize)]
pub struct TlsConfig {
    // tls server certificate file path
    tls_certificate: PathBuf,
    // tls server private key file path
    tls_key: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            connections_limit: kvsd::default::MAX_TCP_CONNECTIONS,
            buffer_size_per_connection: kvsd::default::BUFFER_SIZE_PER_CONNECTION,
            authenticate_timeout: kvsd::default::AUTHENTICATE_TIMEOUT,
            bind_address: kvsd::default::bind_address(),
            bind_port: kvsd::default::BIND_PORT,
            tls: kvsd::default::TLS_CONNECTION,
        }
    }
}

impl Config {
    fn merge_config_file(&mut self, _file: ConfigFile) {
        todo!()
    }

    fn merge_args(&mut self, _flags: KvsdOptions) {
        todo!()
    }
}
