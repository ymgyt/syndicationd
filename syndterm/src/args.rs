use std::path::PathBuf;

use clap::Parser;
use url::Url;

use crate::config;

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, about = "xxx")]
pub struct Args {
    /// syndapi endpoint
    #[arg(long, default_value = config::api::ENDPOINT)]
    pub endpoint: Url,
    /// Log file path
    #[arg(long, default_value = config::log_path().into_os_string())]
    pub log: PathBuf,
}

pub fn parse() -> Args {
    Args::parse()
}
