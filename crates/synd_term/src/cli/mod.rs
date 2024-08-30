use std::{path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{config, ui::theme};

mod command;
mod port;

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum Palette {
    Ferra,
    SolarizedDark,
    Helix,
}

impl From<Palette> for theme::Palette {
    fn from(p: Palette) -> Self {
        match p {
            Palette::Ferra => theme::Palette::ferra(),
            Palette::SolarizedDark => theme::Palette::solarized_dark(),
            Palette::Helix => theme::Palette::helix(),
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, name = "synd")]
pub struct Args {
    /// Configuration file path
    #[arg(long, short = 'c', env = config::env::CONFIG_FILE)]
    pub config: Option<PathBuf>,
    /// Log file path
    #[arg(long, env = config::env::LOG_FILE)]
    pub log: Option<PathBuf>,
    /// Cache directory
    #[arg(long, env = config::env::CACHE_DIR)]
    pub cache_dir: Option<PathBuf>,
    /// Color theme
    #[arg(value_enum, long = "theme", env = config::env::THEME, value_name = "THEME")]
    pub palette: Option<Palette>,
    #[command(subcommand)]
    pub command: Option<Command>,
    #[command(flatten)]
    pub api: ApiOptions,
    #[command(flatten)]
    pub feed: FeedOptions,
    #[command(flatten)]
    pub github: GithubOptions,
    #[arg(hide = true, long = "dry-run", hide_long_help = true)]
    pub dry_run: bool,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Api options")]
pub struct ApiOptions {
    /// `synd_api` endpoint
    #[arg(long, global = true, env = config::env::ENDPOINT)]
    pub endpoint: Option<Url>,
    /// Client timeout(ex. 30s)
    #[arg(long, value_parser = config::parse::flag::parse_duration_opt, env = config::env::CLIENT_TIMEOUT)]
    pub client_timeout: Option<Duration>,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Feed options")]
pub struct FeedOptions {
    /// Feed entries limit to fetch
    #[arg(long, aliases = ["max-entries"], env = config::env::FEED_ENTRIES_LIMIT)]
    pub entries_limit: Option<usize>,
    /// Browser command to open feed entry
    #[arg(long, env = config::env::FEED_BROWSER)]
    pub browser: Option<PathBuf>,
    /// Args for launching the browser command
    #[arg(long, env = config::env::FEED_BROWSER_ARGS)]
    pub browser_args: Option<Vec<String>>,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "GitHub options")]
pub struct GithubOptions {
    /// Enable GitHub notification feature
    #[arg(
        long,
        short = 'G',
        visible_alias = "enable-gh",
        env = config::env::ENABLE_GITHUB,
    )]
    pub enable_github_notification: Option<bool>,
    /// GitHub personal access token to fetch notifications
    #[arg(
        long,
        env = config::env::GITHUB_PAT,
        hide_env_values = true,
    )]
    pub github_pat: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(alias = "clear")]
    Clean(command::clean::CleanCommand),
    Check(command::check::CheckCommand),
    Export(command::export::ExportCommand),
    Import(command::import::ImportCommand),
    Config(command::config::ConfigCommand),
}

pub fn parse() -> Args {
    Args::parse()
}
