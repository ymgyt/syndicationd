use std::{path::PathBuf, process::ExitCode};

use anyhow::Context as _;
use synd_runtime::Session;
use synd_support::fs::fsimpl::FileSystem;
use synd_term::{
    application::{Application, Cache, Config, Features, FeedBackend},
    client::github::GithubClient,
    interact::{ProcessInteractor, TextBrowserInteractor},
    terminal::{self, Terminal},
    ui::theme::Theme,
};
use tracing::error;
use tracing_appender::non_blocking::WorkerGuard;

use crate::{cli::Args, config::ConfigResolver, runtime::FeedRuntime};

mod cli;
mod config;
mod runtime;

fn init_tracing(
    log_path: Option<PathBuf>,
    default_filter: &'static str,
    stderr: bool,
) -> anyhow::Result<Option<WorkerGuard>> {
    use synd_support::o11y::opentelemetry::init_propagation;
    use tracing_subscriber::{
        Registry,
        filter::EnvFilter,
        fmt::{self, writer::BoxMakeWriter},
        layer::SubscriberExt,
        util::SubscriberInitExt as _,
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
    } else if stderr {
        (BoxMakeWriter::new(std::io::stderr), None)
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

#[allow(clippy::needless_pass_by_value)]
async fn build_app(
    config: ConfigResolver,
    dry_run: bool,
) -> anyhow::Result<(Application, Session)> {
    let session = FeedRuntime::new(&config).acquire_session().await?;
    let feed_backend = FeedBackend::established(session.client().clone());

    let mut builder = Application::builder()
        .terminal(Terminal::new().context("Failed to construct terminal")?)
        .feed_backend(feed_backend)
        .categories(config.categories())
        .config(Config {
            entries_limit: config.feed_entries_limit(),
            features: Features {
                enable_github_notification: config.is_github_enable(),
            },
            ..Default::default()
        })
        .cache(Cache::new(config.cache_dir()))
        .theme(Theme::with_palette(config.palette()))
        .interactor(Box::new(ProcessInteractor::new(
            TextBrowserInteractor::new(config.feed_browser_command(), config.feed_browser_args()),
        )))
        .dry_run(dry_run);

    if config.is_github_enable() {
        builder = builder.github_client(
            GithubClient::new(config.github_pat()).context("Failed to construct github client")?,
        );
    }

    Ok((builder.build(), session))
}

#[tokio::main]
async fn main() -> ExitCode {
    crypto_provider::init();

    let (config, command, dry_run) = {
        let Args {
            config,
            log,
            cache_dir,
            api,
            backend,
            feed,
            github,
            command,
            palette,
            dry_run,
        } = cli::parse();

        let config = match ConfigResolver::builder()
            .config_file(config)
            .log_file(log)
            .cache_dir(cache_dir)
            .api_options(api)
            .backend_options(backend)
            .feed_options(feed)
            .github_options(github)
            .palette(palette)
            .try_build()
        {
            Ok(config) => config,
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::FAILURE;
            }
        };
        (config, command, dry_run)
    };

    let _guard = {
        let is_subcommand = command.is_some();
        let log = if is_subcommand {
            None
        } else {
            Some(config.log_file())
        };
        let default_filter = if is_subcommand { "warn" } else { "info" };
        match init_tracing(log, default_filter, is_subcommand) {
            Ok(guard) => guard,
            Err(err) => {
                eprintln!("{err:?}");
                return ExitCode::FAILURE;
            }
        }
    };

    if let Some(command) = command {
        return match command {
            cli::Command::Clean(clean) => clean.run(&config, &FileSystem::new()),
            cli::Command::Doctor(doctor) => doctor.run(config).await,
            cli::Command::Feed(feed) => feed.run(config).await,
            cli::Command::Config(command) => command.run(&config),
        };
    }

    let mut event_stream = terminal::event_stream();

    if let Err(err) = async {
        let (app, session) = build_app(config, dry_run).await?;
        let result = {
            tracing::info!("Running...");
            app.run(&mut event_stream)
        }
        .await;

        if let Err(err) = session.close().await {
            tracing::warn!("Failed to close runtime session: {err}");
        }

        result
    }
    .await
    {
        error!("{err:?}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
