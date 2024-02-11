use std::path::PathBuf;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, propagate_version = true, disable_help_subcommand = true)]
pub struct Args {
    #[command(flatten)]
    pub kvsd: KvsdOptions,
    #[command(flatten)]
    pub tls: TlsOptions,
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

#[must_use]
pub fn parse() -> Args {
    Args::parse()
}
