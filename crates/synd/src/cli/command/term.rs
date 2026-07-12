use std::process::ExitCode;

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

        info!("Running...");
        let result = app.run(&mut event_stream).await;

        if let Err(err) = session.close().await {
            warn!("Failed to close runtime session: {err}");
        }

        if let Err(err) = result {
            error!("{err:?}");
            ExitCode::FAILURE
        } else {
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
