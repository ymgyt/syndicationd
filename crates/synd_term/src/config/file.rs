use std::{collections::HashMap, io, path::PathBuf, time::Duration};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use crate::{cli::Palette, config::categories};

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub(super) directory: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    pub(super) path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThemeEntry {
    pub(super) name: Option<Palette>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedEntry {
    pub(super) entries_limit: Option<usize>,
    pub(super) browser: Option<FeedBrowserEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedBrowserEntry {
    pub(super) command: Option<PathBuf>,
    pub(super) args: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiEntry {
    pub(super) endpoint: Option<Url>,
    #[serde(
        default,
        deserialize_with = "synd_stdx::time::humantime::de::parse_duration_opt"
    )]
    pub(super) timeout: Option<Duration>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubEntry {
    pub(super) enable: Option<bool>,
    pub(super) pat: Option<String>,
}

#[derive(Error, Debug)]
pub enum ConfigFileError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    Deserialize(#[from] toml::de::Error),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    pub(super) cache: Option<CacheEntry>,
    pub(super) log: Option<LogEntry>,
    pub(super) theme: Option<ThemeEntry>,
    pub(super) api: Option<ApiEntry>,
    pub(super) feed: Option<FeedEntry>,
    pub(super) github: Option<GithubEntry>,
    pub(super) categories: Option<HashMap<String, categories::Entry>>,
}

impl ConfigFile {
    pub(super) fn new<R: io::Read>(mut src: R) -> Result<Self, ConfigFileError> {
        let mut buf = String::new();
        src.read_to_string(&mut buf)?;
        toml::from_str(&buf).map_err(ConfigFileError::from)
    }
}

pub static INIT_CONFIG: &str = r#"
[cache]
# Cache directory
# directory = "path/to/dir"

[log]
# Log file path
# path = "path/to/log"

[theme]
# Theme name 
# The available themes can be found by `synd --help`
# name = "ferra"

[api]
# Backend api endpoint
# endpoint = "https://api.syndicationd.ymgyt.io"

# Client timeout duration 
# timeout = "30s"

[feed]
# Feed entries to fetch
# entries_limit = 200 

# Command to browse feed
# browser = { command = "", args = [] }

[github]
# Enable github notification feature
# enable = true

# Github Personal access token(PAT) to browse notifications
# pat = "ghp_xxxx"
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        let src = r#"
[cache]
directory = "/tmp/synd/cache"

[log]
path = "/tmp/synd/synd.log"

[theme]
name = "ferra"

[api]
endpoint = "https://api.syndicationd.ymgyt.io"
timeout = "30s"

[feed]
entries_limit = 100
browser = { command = "w3m", args = ["--foo", "--bar"] }

[github]
enable = true
pat = "ghp_xxxx"

[categories.rust]
icon = { symbol = "S", color = { rgb = 0xF74C00 }}
aliases = ["rs"]
"#;

        let config = ConfigFile::new(src.as_bytes()).unwrap();

        insta::assert_debug_snapshot!("deserialized_config", config);
    }
}
