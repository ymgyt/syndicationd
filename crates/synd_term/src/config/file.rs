use std::{path::PathBuf, time::Duration};

use serde::{Deserialize, Deserializer, Serialize};
use url::Url;

use crate::cli::Palette;

#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    dir: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    path: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiEntry {
    endpoint: Option<Url>,
    #[serde(deserialize_with = "parse_duration")]
    timeout: Option<Duration>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigFile {
    cache: Option<CacheEntry>,
    log: Option<LogEntry>,
    theme: Option<Palette>,
    api: Option<ApiEntry>,
}

fn parse_duration<'de, D>(deserializer: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    todo!()
}
