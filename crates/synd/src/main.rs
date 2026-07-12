use std::{io::IsTerminal as _, process::ExitCode};

use synd_support::fs::fsimpl::FileSystem;
use tracing_appender::non_blocking::WorkerGuard;

use crate::config::ConfigResolver;

mod cli;
mod config;
mod release;
mod runtime;

fn init_tracing(
    config: &ConfigResolver,
    command: &cli::Command,
) -> anyhow::Result<Option<WorkerGuard>> {
    use synd_support::o11y::{
        opentelemetry::init_propagation, tracing_subscriber::otel_metrics::metrics_event_filter,
    };
    use tracing_subscriber::{
        Layer as _, Registry,
        filter::EnvFilter,
        fmt::{self, writer::BoxMakeWriter},
        layer::SubscriberExt,
        util::SubscriberInitExt as _,
    };

    let default_filter = match command {
        cli::Command::Term(_) | cli::Command::Daemon(_) => "info",
        cli::Command::Clean(_)
        | cli::Command::Doctor(_)
        | cli::Command::Feed(_)
        | cli::Command::Config(_) => "warn",
    };

    let (writer, guard, enable_ansi) = match command {
        cli::Command::Term(_) => {
            let log_path = config.log_file();
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let log = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(log_path)?;
            let (non_blocking, guard) = tracing_appender::non_blocking(log);
            (BoxMakeWriter::new(non_blocking), Some(guard), false)
        }
        _ => (
            BoxMakeWriter::new(std::io::stderr),
            None,
            std::io::stderr().is_terminal(),
        ),
    };

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(enable_ansi)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(false)
                .with_line_number(false)
                .with_target(true)
                .with_writer(writer)
                .with_filter(metrics_event_filter()),
        )
        .with(
            EnvFilter::try_from_env(config::env::LOG_DIRECTIVE)
                .or_else(|_| EnvFilter::try_new(default_filter))
                .unwrap(),
        )
        .try_init()?;

    init_propagation();

    Ok(guard)
}

mod crypto_provider {
    use rustls::crypto::CryptoProvider;

    pub(super) fn init() {
        try_init().expect("failed to initialize rustls CryptoProvider");
    }

    pub(super) fn try_init() -> anyhow::Result<()> {
        if CryptoProvider::get_default().is_some() {
            return Ok(());
        }

        rustls::crypto::ring::default_provider()
            .install_default()
            .map_err(|_| anyhow::anyhow!("failed to install rustls ring CryptoProvider"))
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    crypto_provider::init();

    let (config_builder, command) = cli::parse();
    let config = match config_builder.try_build() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::FAILURE;
        }
    };

    let _guard = match init_tracing(&config, &command) {
        Ok(guard) => guard,
        Err(err) => {
            eprintln!("{err:?}");
            return ExitCode::FAILURE;
        }
    };

    match command {
        cli::Command::Term(term) => return term.run(config).await,
        cli::Command::Clean(clean) => return clean.run(&config, &FileSystem::new()),
        cli::Command::Daemon(daemon) => return daemon.run(config).await,
        cli::Command::Doctor(doctor) => return doctor.run(config).await,
        cli::Command::Feed(feed) => return feed.run(config).await,
        cli::Command::Config(command) => return command.run(&config),
    }
}
