use std::{net::IpAddr, time::Duration};

use serde::Deserialize;
use synd_stdx::byte::Byte;

#[derive(Debug, Deserialize)]
pub(crate) struct ConfigFile {
    connection: Option<ConnectionEntry>,
    authentication: Option<AuthenticationEntry>,
    bind: Option<BindEntry>,
    tls: Option<TlsEntry>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConnectionEntry {
    limit: Option<u32>,
    buffer_size: Option<Byte>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthenticationEntry {
    #[serde(
        default,
        deserialize_with = "synd_stdx::time::humantime::de::parse_duration_opt"
    )]
    timeout: Option<Duration>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BindEntry {
    address: Option<IpAddr>,
    port: Option<u16>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TlsEntry {
    disable: Option<bool>,
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
