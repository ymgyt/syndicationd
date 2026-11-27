use std::io::{self, IsTerminal as _};

use anyhow::{Context as _, ensure};
use synd_term::{
    application::{Application, ApplicationBuilder, Cache, Config, Features, FeedClient},
    client::gh::GhClient,
    config::Categories,
    external_tools::{ExternalTools, SystemTools},
    ui::theme::Theme,
};

use crate::{config::ConfigResolver, release::ReleaseCheck, runtime::FeedRuntime};

type ConfiguredApplicationBuilder =
    ApplicationBuilder<(), Categories, Cache, Config, Theme, Box<dyn ExternalTools>>;

/// Run the terminal UI.
/// This is the default command executed when `synd` is invoked without a subcommand;
/// its options are defined on the top-level CLI(`cli::TermOptions`).
#[derive(Debug)]
pub struct TermCommand;

impl TermCommand {
    pub async fn run(self, config: &ConfigResolver) -> anyhow::Result<()> {
        ensure!(io::stdout().is_terminal(), "stdout is not a terminal");
        let builder = Self::application_builder(config)?;
        let runtime = FeedRuntime::new(config)?;

        let release_check = runtime
            .with_session(async |client| {
                let app = builder.feed_api(FeedClient::new(client.clone())).build();
                let check = ReleaseCheck::spawn();
                app.run().await.map(|()| check)
            })
            .await?;
        release_check.print_notice_if_ready();
        Ok(())
    }

    fn application_builder(
        config: &ConfigResolver,
    ) -> anyhow::Result<ConfiguredApplicationBuilder> {
        let gh_client = if config.is_gh_enabled() {
            Some(GhClient::new(config.gh_pat()).context("Failed to construct GitHub client")?)
        } else {
            None
        };
        let app_config = Config {
            entries_limit: config.feed_entries_limit(),
            features: Features {
                enable_gh_notification: gh_client.is_some(),
            },
            keymaps: config.keymaps(),
            ..Default::default()
        };
        Ok(Application::builder()
            .gh_client(gh_client)
            .categories(config.categories())
            .config(app_config)
            .cache(Cache::new(config.cache_dir()))
            .theme(Theme::new(config.palette()).with_background(config.theme_background))
            .external_tools(SystemTools::new(
                config.feed_browser_command(),
                config.feed_browser_args(),
            )))
    }
}
