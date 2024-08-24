use std::{future, path::PathBuf, process::ExitCode, time::Duration};

use anyhow::Context as _;
use futures_util::TryFutureExt;
use synd_term::{
    application::{Application, Cache, Config, Features},
    cli::{self, ApiOptions, Args, FeedOptions, GithubOptions, Palette},
    client::{github::GithubClient, Client},
    config::{self, Categories},
    filesystem::fsimpl::FileSystem,
    terminal::{self, Terminal},
    ui::theme::Theme,
};
use tracing::error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use url::Url;

fn init_tracing(log_path: Option<PathBuf>) -> anyhow::Result<Option<WorkerGuard>> {
    use synd_o11y::opentelemetry::init_propagation;
    use tracing_subscriber::{
        filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt as _, Registry,
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

fn build_app(
    endpoint: Url,
    timeout: Duration,
    palette: Palette,
    FeedOptions {
        categories,
        entries_limit,
    }: FeedOptions,
    cache_dir: PathBuf,
    GithubOptions {
        github_pat,
        enable_github_notification,
    }: GithubOptions,
    dry_run: bool,
) -> anyhow::Result<Application> {
    let mut builder = Application::builder()
        .terminal(Terminal::new().context("Failed to construct terminal")?)
        .client(Client::new(endpoint, timeout).context("Failed to construct client")?)
        .categories(
            categories
                .map(Categories::load)
                .transpose()?
                .unwrap_or_else(Categories::default_toml),
        )
        .config(Config {
            entries_limit,
            features: Features {
                enable_github_notification,
            },
            ..Default::default()
        })
        .cache(Cache::new(cache_dir))
        .theme(Theme::with_palette(&palette.into()))
        .dry_run(dry_run);

    if enable_github_notification {
        builder = builder.github_client(GithubClient::new(github_pat.unwrap()));
    }

    Ok(builder.build())
}

#[tokio::main]
async fn main() -> ExitCode {
    let Args {
        api: ApiOptions {
            endpoint,
            client_timeout,
        },
        feed,
        log,
        cache_dir,
        command,
        palette,
        dry_run,
        experimental,
    } = cli::parse();

    // Subcommand logs to the terminal, tui writes logs to a file.
    let log = if command.is_some() { None } else { Some(log) };
    let _guard = init_tracing(log).unwrap();

    if let Some(command) = command {
        return match command {
            cli::Command::Clean(clean) => clean.run(&FileSystem::new()),
            cli::Command::Check(check) => check.run(endpoint).await,
            cli::Command::Export(export) => export.run(endpoint).await,
            cli::Command::Import(import) => import.run(endpoint).await,
        };
    };

    let mut event_stream = terminal::event_stream();

    if let Err(err) = future::ready(build_app(
        endpoint,
        client_timeout,
        palette,
        feed,
        cache_dir,
        experimental,
        dry_run,
    ))
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
