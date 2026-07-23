use std::{process::ExitCode, time::Instant};

use anyhow::Context as _;
use synd_runtime::Session;
use synd_term::{
    application::{Application, Cache, ClientFeedApi, Config, Features},
    client::github::GithubClient,
    interact::{ProcessInteractor, TextBrowserInteractor},
    terminal::{self, Terminal},
    ui::theme::Theme,
};
use tracing::{error, info, warn};

use crate::{config::ConfigResolver, release, runtime::FeedRuntime};

/// Run the terminal UI.
/// This is the default command executed when `synd` is invoked without a subcommand;
/// its options are defined on the top-level CLI(`cli::TermOptions`).
#[derive(Debug)]
pub struct TermCommand {
    dry_run: bool,
}

impl TermCommand {
    pub(crate) fn new(dry_run: bool) -> Self {
        Self { dry_run }
    }

    pub async fn run(self, config: ConfigResolver) -> ExitCode {
        let log_file = config.log_file();
        let dry_run = self.dry_run;
        info!(
            config_file = %config.config_file().display(),
            log_file = %log_file.display(),
            database = %config.sqlite_db().display(),
            runtime_root = ?config.daemon_runtime_root(),
            api_timeout_ms = config.api_timeout().as_millis(),
            session_lease_ms = config.daemon_session_lease_duration().as_millis(),
            idle_shutdown_grace_ms = config.daemon_session_idle_shutdown_grace().as_millis(),
            entries_limit = config.feed_entries_limit(),
            github_enabled = config.is_github_enable(),
            dry_run,
            "Resolved terminal configuration"
        );
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

        let started_at = Instant::now();
        info!(dry_run, "Terminal UI started");
        let result = app.run(&mut event_stream).await;

        if let Err(err) = session.close().await {
            warn!("Failed to close runtime session: {err}");
        }

        if let Err(err) = result {
            error!("{err:?}");
            info!(
                reason = "error",
                outcome = "failure",
                uptime_ms = started_at.elapsed().as_millis(),
                "Terminal UI stopped"
            );
            ExitCode::FAILURE
        } else {
            info!(
                reason = if dry_run { "dry_run" } else { "user_quit" },
                outcome = "success",
                uptime_ms = started_at.elapsed().as_millis(),
                "Terminal UI stopped"
            );
            release_check.print_notice_if_ready();
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

    let mut builder = Application::builder()
        .terminal(terminal)
        .feed_api(ClientFeedApi::new(session.client().clone()))
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
