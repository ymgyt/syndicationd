use std::{ffi::OsString, path::PathBuf, time::Duration};

use synd_stdx::time::humantime;

use crate::config;
use clap::{ArgAction, Parser};

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, disable_help_subcommand = true)]
pub struct Args {
    #[command(flatten)]
    pub kvsd: KvsdOptions,
    #[command(flatten)]
    pub o11y: ObservabilityOptions,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Kvsd options")]
pub struct KvsdOptions {
    /// Max tcp connections
    #[arg(long, env = config::env::MAX_CONNECTIONS)]
    pub(crate) max_connections: Option<u32>,
    /// Buffer bytes assigned to each connection
    #[arg(long, env = config::env::CONNECTION_BUFFER_BYTES)]
    pub(crate) connection_buffer_bytes: Option<usize>,
    /// Authenticate timeout
    #[arg(long, value_parser = humantime::parse_duration, env = config::env::AUTHENTICATE_TIMEOUT)]
    pub(crate) authenticate_timeout: Option<Duration>,
    /// Configuration file path
    // TODO: use toml
    #[arg(long, short = 'C', env = config::env::CONFIG_FILE)]
    pub(crate) config: Option<PathBuf>,
    /// Tcp binding address host(e.g. 0.0.0.0, localhost)
    // TODO: use Url or Addr
    #[arg(long, env = config::env::BIND_ADDRESS)]
    pub(crate) bind_address: Option<String>,
    /// Tcp binding address port
    #[arg(long, env = config::env::BIND_PORT)]
    pub(crate) bind_port: Option<u16>,
    /// Root directory where kvsd store it's data
    #[arg(long, env = config::env::DATA_DIR, default_value = ".kvsd")]
    pub(crate) data_dir: PathBuf,
    /// Tls server certificate file path
    #[arg(long, env = config::env::TLS_CERT, default_value = "./files/localhost.pem")]
    pub(crate) tls_cert: PathBuf,
    /// Tls server private key file path
    #[arg(long, env = config::env::TLS_KEY, default_value = "./files/localhost.key")]
    pub(crate) tls_key: PathBuf,
    /// Disable Tls
    #[arg(long, env = config::env::DISABLE_TLS)]
    pub(crate) disable_tls: bool,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Observability options")]
pub struct ObservabilityOptions {
    /// Show code location(file, line number) in logs
    #[arg(long, env = config::env::LOG_SHOW_LOCATION, default_value_t = false, action = ArgAction::Set )]
    pub show_code_location: bool,

    /// Show event target(module in default) in logs
    #[arg(long, env = config::env::LOG_SHOW_TARGET, default_value_t = true, action = ArgAction::Set)]
    pub show_target: bool,

    /// Opentelemetry otlp exporter endpoint
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,

    /// Opentelemetry trace sampler ratio
    #[arg(long, env = "OTEL_TRACES_SAMPLER_ARG", default_value_t = 1.0)]
    pub trace_sampler_ratio: f64,
}

pub fn try_parse<I, T>(iter: I) -> Result<Args, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Args::try_parse_from(iter)
}
