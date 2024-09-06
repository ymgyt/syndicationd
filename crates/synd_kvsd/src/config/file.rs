use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct ConfigFile {
    connection: ConnectionEntry,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ConnectionEntry {
    limit: Option<u32>,
    buffer_size: Option<String>,
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
