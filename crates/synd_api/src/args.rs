use std::path::PathBuf;

use clap::{ArgAction, Parser};

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, disable_help_subcommand = true)]
pub struct Args {
    #[command(flatten)]
    pub kvsd: KvsdOptions,
    #[command(flatten)]
    pub tls: TlsOptions,
    #[command(flatten)]
    pub o11y: ObservabilityOptions,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Kvsd options")]
pub struct KvsdOptions {
    #[arg(long = "kvsd-host", env = "SYND_KVSD_HOST")]
    pub host: String,
    #[arg(long = "kvsd-port", env = "SYND_KVSD_PORT")]
    pub port: u16,
    #[arg(long = "kvsd-username", alias = "kvsd-user", env = "SYND_KVSD_USER")]
    pub username: String,
    #[arg(long = "kvsd-password", alias = "kvsd-pass", env = "SYND_KVSD_PASS")]
    pub password: String,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Tls options")]
pub struct TlsOptions {
    /// Tls certificate file path
    #[arg(long = "tls-cert", env = "SYND_TLS_CERT", value_name = "CERT_PATH")]
    pub certificate: PathBuf,
    /// Tls private key file path
    #[arg(long = "tls-key", env = "SYND_TLS_KEY", value_name = "KEY_PATH")]
    pub private_key: PathBuf,
}

#[derive(clap::Args, Debug)]
#[command(next_help_heading = "Observability options")]
pub struct ObservabilityOptions {
    /// Show code location(file, line number) in logs
    #[arg(long, env = "SYND_LOG_SHOW_LOCATION", default_value_t = false, action = ArgAction::Set )]
    pub show_code_location: bool,

    /// Show event target(module in default) in logs
    #[arg(long, env = "SYND_LOG_SHOW_TARGET", default_value_t = true, action = ArgAction::Set)]
    pub show_target: bool,

    /// Opentelemetry otlp exporter endpoint
    #[arg(long, env = "OTEL_EXPORTER_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,

    /// Opentelemetry trace sampler ratio
    #[arg(long, env = "OTEL_TRACES_SAMPLER_ARG", default_value_t = 1.0)]
    pub trace_sampler_ratio: f64,
}

#[must_use]
pub fn parse() -> Args {
    Args::parse()
}
