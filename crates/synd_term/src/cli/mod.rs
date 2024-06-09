use std::{path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};
use url::Url;

use crate::{config, ui::theme};

mod check;
mod clean;
mod export;

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum)]
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
    /// Log file path
    #[arg(long, default_value = config::log_path().into_os_string(), env = config::env::LOG_PATH)]
    pub log: PathBuf,
    /// Cache directory
    #[arg(
        long,
        default_value = config::cache::dir().to_path_buf().into_os_string(),
    )]
    pub cache_dir: PathBuf,
    /// Color theme
    #[arg(value_enum, long = "theme", default_value_t = Palette::Ferra, env = config::env::THEME, value_name = "THEME")]
    pub palette: Palette,
    #[command(subcommand)]
    pub command: Option<Command>,
    #[command(flatten)]
    pub api: ApiOptions,
    #[command(flatten)]
    pub feed: FeedOptions,
    #[arg(hide = true, long = "dry-run", hide_long_help = true)]
    pub dry_run: bool,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Api options")]
pub struct ApiOptions {
    /// `synd_api` endpoint
    #[arg(long, global = true, default_value = config::api::ENDPOINT, env = config::env::ENDPOINT)]
    pub endpoint: Url,
    /// Client timeout
    #[arg(long, value_parser = parse_duration::parse, default_value = config::client::DEFAULT_TIMEOUT)]
    pub client_timeout: Duration,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Feed options")]
pub struct FeedOptions {
    /// categories.toml path
    #[arg(long,aliases = ["category"],value_name = "CATEGORIES TOML PATH")]
    pub categories: Option<PathBuf>,
    /// Feed entries limit to fetch
    #[arg(long, aliases = ["max-entries"], default_value_t = config::feed::DEFAULT_ENTRIES_LIMIT)]
    pub entries_limit: usize,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    #[command(alias = "clear")]
    Clean(clean::CleanCommand),
    Check(check::CheckCommand),
    Export(export::ExportCommand),
}

pub fn parse() -> Args {
    Args::parse()
}
