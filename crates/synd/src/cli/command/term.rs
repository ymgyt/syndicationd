use std::process::ExitCode;

use anyhow::Context as _;
use clap::Args;
use synd_runtime::Session;
use synd_term::{
    application::{Application, Cache, Config, Features, FeedBackend},
    client::github::GithubClient,
    interact::{ProcessInteractor, TextBrowserInteractor},
    terminal::{self, Terminal},
    ui::theme::Theme,
};
use tracing::{error, info, warn};

use crate::{
    cli::{FeedOptions, GithubOptions, Palette},
    config::{self, ConfigResolver},
    release,
    runtime::FeedRuntime,
};

/// Run terminal UI
#[derive(Args, Clone, Debug)]
#[command(next_help_heading = "Term options")]
pub struct TermCommand {
    /// Color theme
    #[arg(value_enum, long = "theme", env = config::env::THEME, value_name = "THEME")]
    pub palette: Option<Palette>,
    #[command(flatten)]
    pub feed: FeedOptions,
    #[command(flatten)]
    pub github: GithubOptions,
    #[arg(hide = true, long = "dry-run", hide_long_help = true)]
    pub dry_run: bool,
}

impl TermCommand {
    #[expect(clippy::large_futures)]
    pub async fn run(self, config: ConfigResolver) -> ExitCode {
        let log_file = config.log_file();
        let (app, session) = match build_app(config, self.dry_run).await {
            Ok(started) => started,
            Err(err) => {
                error!("{err:?}");
                eprintln!("error: {err:#}");
                eprintln!("see log: {}", log_file.display());
                return ExitCode::FAILURE;
            }
        };

        let mut event_stream = terminal::event_stream();
        let release_check = release::ReleaseCheck::spawn();
        let result = {
            info!("Running...");
            app.run(&mut event_stream)
        }
        .await;

        if let Err(err) = session.close().await {
            warn!("Failed to close runtime session: {err}");
        }

        release_check.print_notice_if_ready();

        if let Err(err) = result {
            error!("{err:?}");
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}

async fn build_app(
    config: ConfigResolver,
    dry_run: bool,
) -> anyhow::Result<(Application, Session)> {
    let terminal = Terminal::new().context("Failed to construct terminal")?;
    let github_client = if config.is_github_enable() {
        Some(GithubClient::new(config.github_pat()).context("Failed to construct github client")?)
    } else {
        None
    };
    let session = FeedRuntime::new(&config)?.acquire_session().await?;
    let feed_backend = FeedBackend::established(session.client().clone());

    let mut builder = Application::builder()
        .terminal(terminal)
        .feed_backend(feed_backend)
        .categories(config.categories())
        .config(Config {
            entries_limit: config.feed_entries_limit(),
            features: Features {
                enable_github_notification: config.is_github_enable(),
            },
            keymaps: config.keymaps(),
            ..Default::default()
        })
        .cache(Cache::new(config.cache_dir()))
        .theme(Theme::with_palette(config.palette()))
        .interactor(Box::new(ProcessInteractor::new(
            TextBrowserInteractor::new(config.feed_browser_command(), config.feed_browser_args()),
        )))
        .dry_run(dry_run);

    if let Some(github_client) = github_client {
        builder = builder.github_client(github_client);
    }

    Ok((builder.build(), session))
}
