use clap::Parser;
use url::Url;

use crate::config;

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, about = "xxx")]
pub struct Args {
    /// syndapi endpoint
    #[arg(default_value = config::api::ENDPOINT)]
    pub endpoint: Url,
}

pub fn parse() -> Args {
    Args::parse()
}
