use std::path::PathBuf;

use clap::{Parser, Subcommand};
use url::Url;

use crate::config;

mod clear;

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, about = "xxx")]
pub struct Args {
    /// synd_api endpoint
    #[arg(long, default_value = config::api::ENDPOINT)]
    pub endpoint: Url,
    /// Log file path
    #[arg(long, default_value = config::log_path().into_os_string())]
    pub log: PathBuf,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Clear(clear::ClearCommand),
}

pub fn parse() -> Args {
    Args::parse()
}
