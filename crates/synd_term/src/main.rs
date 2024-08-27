use std::{future, path::PathBuf, process::ExitCode};

use anyhow::Context as _;
use futures_util::TryFutureExt as _;
use synd_term::{
    application::{Application, Cache, Config, Features},
    cli::{self, Args},
    client::{github::GithubClient, Client},
    config::{self, ConfigResolver},
    filesystem::fsimpl::FileSystem,
    terminal::{self, Terminal},
    ui::theme::Theme,
};
use tracing::error;
use tracing_appender::non_blocking::WorkerGuard;

fn init_tracing(log_path: Option<PathBuf>) -> anyhow::Result<Option<WorkerGuard>> {
    use synd_o11y::opentelemetry::init_propagation;
    use tracing_subscriber::{
        filter::EnvFilter,
        fmt::{self, writer::BoxMakeWriter},
        layer::SubscriberExt,
        util::SubscriberInitExt as _,
        Registry,
    };

    let (writer, guard) = if let Some(log_path) = log_path {
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let log = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(log_path)?;
        let (non_blocking, guard) = tracing_appender::non_blocking(log);
        (BoxMakeWriter::new(non_blocking), Some(guard))
    } else {
        (BoxMakeWriter::new(std::io::stdout), None)
    };

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(true)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(false)
                .with_line_number(false)
                .with_target(true)
                .with_writer(writer),
        )
        .with(
            EnvFilter::try_from_env(config::env::LOG_DIRECTIVE)
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .try_init()?;

    // Set text map progator globally
    init_propagation();

    Ok(guard)
}

#[allow(clippy::needless_pass_by_value)]
fn build_app(config: ConfigResolver, dry_run: bool) -> anyhow::Result<Application> {
    let mut builder = Application::builder()
        .terminal(Terminal::new().context("Failed to construct terminal")?)
        .client(
            Client::new(config.api_endpoint(), config.api_timeout())
                .context("Failed to construct client")?,
        )
        .config(Config {
            entries_limit: config.feed_entries_limit(),
            features: Features {
                enable_github_notification: config.is_github_enable(),
            },
            ..Default::default()
        })
        .cache(Cache::new(config.cache_dir()))
        .theme(Theme::with_palette(config.palette()))
        .categories(config.categories())
        .dry_run(dry_run);

    if config.is_github_enable() {
        builder = builder.github_client(GithubClient::new(config.github_pat()));
    }

    Ok(builder.build())
}

#[tokio::main]
async fn main() -> ExitCode {
    let Args {
        config,
        log,
        cache_dir,
        api,
        feed,
        github,
        command,
        palette,
        dry_run,
    } = cli::parse();

    let config = ConfigResolver::builder()
        .config_file(config)
        .log_file(log)
        .cache_dir(cache_dir)
        .api_options(api)
        .feed_options(feed)
        .github_options(github)
        .palette(palette)
        .build();

    // Subcommand logs to the terminal, while tui writes logs to a file.
    let log = if command.is_some() {
        None
    } else {
        Some(config.log_file())
    };
    let _guard = init_tracing(log).unwrap();

    if let Some(command) = command {
        return match command {
            cli::Command::Clean(clean) => clean.run(&FileSystem::new()),
            cli::Command::Check(check) => check.run(config).await,
            cli::Command::Export(export) => export.run(config.api_endpoint()).await,
            cli::Command::Import(import) => import.run(config.api_endpoint()).await,
            cli::Command::Config(config) => config.run(),
        };
    };

    let mut event_stream = terminal::event_stream();

    if let Err(err) = future::ready(build_app(config, dry_run))
        .and_then(|app| {
            tracing::info!("Running...");
            app.run(&mut event_stream)
        })
        .await
    {
        error!("{err:?}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
