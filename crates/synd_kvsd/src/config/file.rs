#![expect(dead_code)]
use std::{io, net::IpAddr, path::Path, time::Duration};

use serde::Deserialize;
use synd_stdx::byte::Byte;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigFileError {
    #[error("open config file: {0}")]
    Open(#[from] io::Error),
    #[error("deserialize config file: {0}")]
    Deserialize(#[from] toml::de::Error),
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConnectionEntry {
    pub(super) limit: Option<u32>,
    pub(super) buffer_size: Option<Byte>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthenticationEntry {
    #[serde(
        default,
        deserialize_with = "synd_stdx::time::humantime::de::parse_duration_opt"
    )]
    pub(super) timeout: Option<Duration>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BindEntry {
    pub(super) address: Option<IpAddr>,
    pub(super) port: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TlsEntry {
    pub(super) disable: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConfigFile {
    pub(super) connection: Option<ConnectionEntry>,
    pub(super) authentication: Option<AuthenticationEntry>,
    pub(super) bind: Option<BindEntry>,
    pub(super) tls: Option<TlsEntry>,
}

impl ConfigFile {
    pub(crate) fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigFileError> {
        let buf = std::fs::read_to_string(path)?;
        toml::from_str(&buf).map_err(ConfigFileError::from)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"
[connection]        
limit = 2048
buffer_size = "4MiB"

[authentication]
timeout = "3s"

[bind]
address = "127.0.0.1"
port = 7777

[tls]
disable = true

"#;

    #[test]
    fn deserialize() {
        let c: ConfigFile = toml::from_str(CONFIG).unwrap();
        insta::assert_debug_snapshot!("deserialized_config_file", c);
    }
}
