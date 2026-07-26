use std::{path::PathBuf, time::Duration};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

use synd_term::ui::theme;

use crate::config::{self, ConfigResolver, ConfigResolverBuilder};

mod command;
mod port;

use command::term::TermCommand;

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "kebab-case"))]
pub enum Palette {
    Dracula,
    Eldritch,
    Ferra,
    SolarizedDark,
    Helix,
}

impl From<Palette> for theme::Palette {
    fn from(p: Palette) -> Self {
        match p {
            Palette::Dracula => theme::Palette::dracula(),
            Palette::Eldritch => theme::Palette::eldritch(),
            Palette::Ferra => theme::Palette::ferra(),
            Palette::SolarizedDark => theme::Palette::solarized_dark(),
            Palette::Helix => theme::Palette::helix(),
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, name = "synd")]
struct Args {
    /// Configuration file path
    #[arg(long, short = 'c', env = config::env::CONFIG_FILE)]
    config: Option<PathBuf>,
    /// Log file path
    #[arg(long, env = config::env::LOG_FILE)]
    log: Option<PathBuf>,
    /// Cache directory
    #[arg(long, env = config::env::CACHE_DIR)]
    cache_dir: Option<PathBuf>,
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    api: ApiOptions,
    #[command(flatten)]
    daemon: DaemonOptions,
    #[command(flatten)]
    backend: BackendOptions,
    #[command(flatten)]
    term: TermOptions,
}

/// Configuration inputs for the terminal UI, the default command of `synd`.
/// Defined only at the top level: the terminal UI is the program itself,
/// not a subcommand. Every field is resolved by `ConfigResolver`.
#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Term options")]
pub struct TermOptions {
    /// Color theme
    #[arg(value_enum, long = "theme", env = config::env::THEME, value_name = "THEME")]
    pub palette: Option<Palette>,
    #[command(flatten)]
    pub feed: FeedOptions,
    #[command(flatten)]
    pub gh: GhOptions,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Api options")]
pub struct ApiOptions {
    /// Client timeout(ex. 30s)
    #[arg(long, value_parser = config::parse::flag::parse_duration_opt, env = config::env::CLIENT_TIMEOUT)]
    pub client_timeout: Option<Duration>,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Daemon options")]
pub struct DaemonOptions {
    /// Runtime artifact root for daemon socket and startup lock
    #[arg(long = "runtime-root", env = config::env::RUNTIME_ROOT, global = true)]
    pub runtime_root: Option<PathBuf>,
    /// Session lease duration granted by the local daemon
    #[arg(long, value_parser = config::parse::flag::parse_duration_opt, env = config::env::DAEMON_SESSION_LEASE_DURATION)]
    pub daemon_session_lease_duration: Option<Duration>,
    /// Grace period before the local daemon shuts down after all sessions are gone
    #[arg(long, value_parser = config::parse::flag::parse_duration_opt, env = config::env::DAEMON_SESSION_IDLE_SHUTDOWN_GRACE)]
    pub daemon_session_idle_shutdown_grace: Option<Duration>,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Backend options")]
pub struct BackendOptions {
    /// `SQLite` database path
    #[arg(long = "sqlite-db", env = config::env::SQLITE_DB, global = true)]
    pub sqlite_db: Option<PathBuf>,
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
pub struct GhOptions {
    /// Enable GitHub notification feature
    #[arg(
        long = "enable-github-notification",
        short = 'G',
        visible_alias = "enable-gh",
        env = config::env::ENABLE_GITHUB,
        value_name = "ENABLE_GITHUB_NOTIFICATION",
    )]
    pub enable_gh_notification: Option<bool>,
    /// GitHub personal access token to fetch notifications
    #[arg(
        long = "github-pat",
        env = config::env::GITHUB_PAT,
        hide_env_values = true,
        value_name = "GITHUB_PAT",
    )]
    pub gh_pat: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Not parsed as a subcommand: `synd` without a subcommand resolves to this
    #[command(skip)]
    Term(TermCommand),
    #[command(alias = "clear")]
    Clean(command::clean::CleanCommand),
    Daemon(command::daemon::DaemonCommand),
    Doctor(command::doctor::DoctorCommand),
    Feed(command::feed::FeedCommand),
    Config(command::config::ConfigCommand),
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
}

/// Parse CLI arguments, then decide the command to run and prime the config
/// resolver with the parsed flags.
/// `synd` without a subcommand runs the terminal UI.
pub fn parse() -> (ConfigResolverBuilder, Command) {
    let Args {
        config,
        log,
        cache_dir,
        command,
        api,
        daemon,
        backend,
        term,
    } = Args::parse();

    let command = command.unwrap_or(Command::Term(TermCommand));
    let builder = ConfigResolver::builder()
        .config_file(config)
        .log_file(log)
        .cache_dir(cache_dir)
        .api_options(api)
        .daemon_options(daemon)
        .backend_options(backend)
        .feed_options(term.feed)
        .gh_options(term.gh)
        .palette(term.palette);

    (builder, command)
}
