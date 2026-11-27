use std::{io::IsTerminal as _, process::ExitCode};

use synd_support::fs::fsimpl::FileSystem;
use tracing::error;
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
        | cli::Command::Mcp(_)
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
                .or_else(|_| EnvFilter::try_new(default_filter))?,
        )
        .try_init()?;

    init_propagation();

    Ok(guard)
}

async fn run() -> anyhow::Result<ExitCode> {
    let (config_builder, command) = cli::parse();
    let config = config_builder.try_build()?;
    let _tracing_guard = init_tracing(&config, &command)?;

    match command {
        cli::Command::Term(term) => term.run(&config).await.map_err(|err| {
            error!("{err:?}");
            err.context(format!(
                "terminal UI failed (see log: {})",
                config.log_file().display()
            ))
        }),
        cli::Command::Clean(clean) => clean.run(&config, &FileSystem::new()),
        cli::Command::Daemon(daemon) => daemon.run(&config).await,
        cli::Command::Doctor(doctor) => return doctor.run(&config).await,
        cli::Command::Feed(feed) => feed.run(&config).await,
        cli::Command::Mcp(mcp) => mcp.run(&config).await,
        cli::Command::Config(command) => command.run(&config),
    }?;
    Ok(ExitCode::SUCCESS)
}

#[tokio::main]
async fn main() -> ExitCode {
    synd::install_tls_crypto_provider().expect("rustls crypto provider already installed");

    match run().await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
