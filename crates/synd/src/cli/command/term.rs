use std::io::{self, IsTerminal as _};

use anyhow::Context as _;
use synd_client::Client;
use synd_term::{
    application::{Application, Cache, Config, Features, FeedClient},
    client::gh::GhClient,
    external_tools::SystemTools,
    ui::theme::Theme,
};
use tracing::{error, info, warn};

use crate::{config::ConfigResolver, release::ReleaseCheck, runtime::FeedRuntime};

/// Run the terminal UI.
/// This is the default command executed when `synd` is invoked without a subcommand;
/// its options are defined on the top-level CLI(`cli::TermOptions`).
#[derive(Debug)]
pub struct TermCommand;

impl TermCommand {
    pub async fn run(self, config: &ConfigResolver) -> anyhow::Result<()> {
        self.run_session(config).await.map_err(|err| {
            error!("{err:?}");
            let log = config.log_file();
            err.context(format!("terminal UI failed (see log: {})", log.display()))
        })
    }

    async fn run_session(self, config: &ConfigResolver) -> anyhow::Result<()> {
        anyhow::ensure!(io::stdout().is_terminal(), "stdout is not a terminal");
        let gh_client = if config.is_gh_enabled() {
            Some(GhClient::new(config.gh_pat()).context("Failed to construct GitHub client")?)
        } else {
            None
        };
        let session = FeedRuntime::new(config)?.acquire_session().await?;
        let app = build_app(config, session.client(), gh_client);
        let release_check = ReleaseCheck::spawn();
        info!("Starting terminal UI");
        let result = app.run().await;
        if let Err(err) = session.close().await {
            warn!("Failed to close runtime session: {err}");
        }
        result?;
        release_check.print_notice_if_ready();
        Ok(())
    }
}

fn build_app(config: &ConfigResolver, client: &Client, gh_client: Option<GhClient>) -> Application {
    let app_config = Config {
        entries_limit: config.feed_entries_limit(),
        features: Features {
            enable_gh_notification: gh_client.is_some(),
        },
        keymaps: config.keymaps(),
        ..Default::default()
    };
    Application::builder()
        .feed_api(FeedClient::new(client.clone()))
        .gh_client(gh_client)
        .categories(config.categories())
        .config(app_config)
        .cache(Cache::new(config.cache_dir()))
        .theme(Theme::new(config.palette()).with_background(config.theme_background))
        .external_tools(SystemTools::new(
            config.feed_browser_command(),
            config.feed_browser_args(),
        ))
        .build()
}
