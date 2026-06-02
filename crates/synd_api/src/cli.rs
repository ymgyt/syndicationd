use std::{ffi::OsString, net::IpAddr, path::PathBuf, str::FromStr, time::Duration};

use axum_server::tls_rustls::RustlsConfig;
use clap::{ArgAction, Parser};
use synd_registry::{
    FeedRegistryConfig,
    crawl::policy::{CrawlPolicy, PollingInterval},
};
use synd_support::time::humantime;

use crate::{
    Error, Result as ApiResult,
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
    pub local: LocalOptions,
    #[command(flatten)]
    pub lifecycle: LifecycleOptions,
    #[command(flatten)]
    pub o11y: ObservabilityOptions,
    #[command(flatten)]
    pub feed_crawl: FeedCrawlOptions,
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

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            body_limit_bytes: config::serve::DEFAULT_REQUEST_BODY_LIMIT_BYTES,
            concurrency_limit: config::serve::DEFAULT_REQUEST_CONCURRENCY_LIMIT,
        }
    }
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Tls options")]
pub struct TlsOptions {
    /// Tls certificate file path
    #[arg(long = "tls-cert", env = env_key!("TLS_CERT"), value_name = "CERT_PATH")]
    pub certificate: Option<PathBuf>,
    /// Tls private key file path
    #[arg(long = "tls-key", env = env_key!("TLS_KEY"), value_name = "KEY_PATH")]
    pub private_key: Option<PathBuf>,
}

impl TlsOptions {
    pub async fn rustls_config(&self, local_enabled: bool) -> ApiResult<Option<RustlsConfig>> {
        if local_enabled {
            return Ok(None);
        }

        let certificate = self
            .certificate
            .as_ref()
            .ok_or(Error::TlsOptionRequired { field: "tls cert" })?;
        let private_key = self
            .private_key
            .as_ref()
            .ok_or(Error::TlsOptionRequired { field: "tls key" })?;
        RustlsConfig::from_pem_file(certificate, private_key)
            .await
            .map_err(|source| Error::TlsOptions { source })
            .map(Some)
    }
}

#[derive(clap::Args, Debug, Clone)]
#[command(next_help_heading = "Local options")]
pub struct LocalOptions {
    /// Run as a local API child service
    #[arg(long = "local", default_value_t = false, action = ArgAction::SetTrue, env = env_key!("LOCAL"))]
    pub enabled: bool,
}

#[derive(clap::Args, Debug, Clone)]
#[command(next_help_heading = "Lifecycle options")]
pub struct LifecycleOptions {
    /// Shutdown when stdin reaches EOF
    #[arg(long, default_value_t = false, action = ArgAction::SetTrue)]
    pub shutdown_on_stdin_eof: bool,
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

#[derive(clap::Args, Debug, Clone, Copy)]
#[command(next_help_heading = "Feed crawl options")]
pub struct FeedCrawlOptions {
    #[arg(long, value_parser = parse_crawl_interval, default_value = config::feed_crawl::DEFAULT_CRAWL_INTERVAL, env = env_key!("FEED_CRAWL_INTERVAL"))]
    pub default_feed_crawl_interval: PollingInterval,
}

impl Default for FeedCrawlOptions {
    fn default() -> Self {
        Self {
            default_feed_crawl_interval: PollingInterval::try_from(Duration::from_hours(2))
                .expect("default feed crawl interval is non-zero"),
        }
    }
}

impl FeedCrawlOptions {
    pub fn registry_config(self) -> FeedRegistryConfig {
        FeedRegistryConfig {
            default_crawl_policy: CrawlPolicy::interval(self.default_feed_crawl_interval),
            ..FeedRegistryConfig::default()
        }
    }
}

fn parse_crawl_interval(value: &str) -> Result<PollingInterval, String> {
    humantime::parse_duration(value)
        .map_err(|err| err.to_string())
        .and_then(|duration| PollingInterval::try_from(duration).map_err(|err| err.to_string()))
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

    #[test]
    fn parse_local() {
        let args = try_parse([
            "synd-api",
            "--local",
            "--shutdown-on-stdin-eof",
            "--sqlite-db",
            "synd.db",
            "--tls-cert",
            "cert.pem",
            "--tls-key",
            "key.pem",
        ])
        .unwrap();

        assert!(args.local.enabled);
        assert!(args.lifecycle.shutdown_on_stdin_eof);
    }
}
