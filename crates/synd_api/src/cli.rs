use std::{ffi::OsString, net::IpAddr, path::PathBuf, str::FromStr, time::Duration};

use clap::{ArgAction, Parser};
use synd_stdx::time::humantime;

use crate::{
    config::{self, env::env_key},
    serve,
};

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, disable_help_subcommand = true)]
pub struct Args {
    #[command(flatten)]
    pub sqlite: SqliteOptions,
    #[command(flatten)]
    pub bind: BindOptions,
    #[command(flatten)]
    pub serve: ServeOptions,
    #[command(flatten)]
    pub tls: TlsOptions,
    #[command(flatten)]
    pub o11y: ObservabilityOptions,
    #[command(flatten)]
    pub cache: CacheOptions,
    #[arg(hide = true, long = "dry-run", hide_long_help = true)]
    pub dry_run: bool,
}

#[derive(clap::Args, Debug, Clone)]
#[command(next_help_heading = "sqlite options")]
pub struct SqliteOptions {
    #[arg(long, env = env_key!("SQLITE_DB"))]
    pub sqlite_db: PathBuf,
}

#[derive(clap::Args, Debug, Clone)]
#[command(next_help_heading = "Bind options")]
pub struct BindOptions {
    #[arg(long, value_parser = IpAddr::from_str, default_value = config::serve::DEFAULT_ADDR, env = env_key!("BIND_ADDR"))]
    pub addr: IpAddr,
    #[arg(long, default_value_t = config::serve::DEFAULT_PORT, env = env_key!("BIND_PORT"))]
    pub port: u16,
}

#[derive(clap::Args, Debug, Clone)]
#[command(next_help_heading = "Serve options")]
pub struct ServeOptions {
    /// Request timeout duration
    #[arg(long, value_parser = humantime::parse_duration, default_value = config::serve::DEFAULT_REQUEST_TIMEOUT)]
    pub timeout: Duration,
    /// Request body limit
    #[arg(long, default_value_t = config::serve::DEFAULT_REQUEST_BODY_LIMIT_BYTES)]
    pub body_limit_bytes: usize,
    #[arg(long, default_value_t = config::serve::DEFAULT_REQUEST_CONCURRENCY_LIMIT)]
    pub concurrency_limit: usize,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Tls options")]
pub struct TlsOptions {
    /// Tls certificate file path
    #[arg(long = "tls-cert", env = env_key!("TLS_CERT"), value_name = "CERT_PATH")]
    pub certificate: PathBuf,
    /// Tls private key file path
    #[arg(long = "tls-key", env = env_key!("TLS_KEY"), value_name = "KEY_PATH")]
    pub private_key: PathBuf,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Observability options")]
pub struct ObservabilityOptions {
    /// Show code location(file, line number) in logs
    #[arg(long, env = env_key!("LOG_SHOW_LOCATION"), default_value_t = false, action = ArgAction::Set )]
    pub show_code_location: bool,

    /// Show event target(module in default) in logs
    #[arg(long, env = env_key!("LOG_SHOW_TARGET"), default_value_t = true, action = ArgAction::Set)]
    pub show_target: bool,

    /// Opentelemetry otlp exporter endpoint
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,

    /// Opentelemetry trace sampler ratio
    #[arg(long, env = "OTEL_TRACES_SAMPLER_ARG", default_value_t = 1.0)]
    pub trace_sampler_ratio: f64,
}

#[derive(clap::Args, Debug, Clone)]
#[command(next_help_heading = "Cache options")]
pub struct CacheOptions {
    /// Max feed cache size in MiB
    #[arg(long, default_value_t = config::cache::DEFAULT_FEED_CACHE_SIZE_MB, env = env_key!("FEED_CACHE_SIZE") )]
    pub feed_cache_size_mb: u64,
    #[arg(long, value_parser = humantime::parse_duration, default_value = config::cache::DEFAULT_FEED_CACHE_TTL, env = env_key!("FEED_CACHE_TTL"))]
    pub feed_cache_ttl: Duration,
    #[arg(long, value_parser = humantime::parse_duration, default_value = config::cache::DEFAULT_FEED_CACHE_REFRESH_INTERVAL, env = env_key!("FEED_CACHE_REFRESH_INTERVAL"))]
    pub feed_cache_refresh_interval: Duration,
}

pub fn try_parse<I, T>(iter: I) -> Result<Args, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Args::try_parse_from(iter)
}

impl From<BindOptions> for serve::BindOptions {
    fn from(BindOptions { addr, port }: BindOptions) -> Self {
        Self { port, addr }
    }
}

impl From<ServeOptions> for serve::ServeOptions {
    fn from(
        ServeOptions {
            timeout,
            body_limit_bytes,
            concurrency_limit,
        }: ServeOptions,
    ) -> Self {
        Self {
            timeout,
            body_limit_bytes,
            concurrency_limit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse() {
        assert_eq!(
            try_parse(["synd-api", "--version"]).unwrap_err().kind(),
            clap::error::ErrorKind::DisplayVersion
        );
        assert_eq!(
            try_parse(["synd-api", "--help"]).unwrap_err().kind(),
            clap::error::ErrorKind::DisplayHelp,
        );
    }
}
