use std::io::{self, Write as _};

use clap::{Args, Subcommand};
use synd_client::payload::{SubscribeDisposition, SubscribeFeedInput, UnsubscribeDisposition};
use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::{config::ConfigResolver, runtime::FeedRuntime};

use super::{export::ExportCommand, import::ImportCommand};

/// Manage feeds
#[derive(Args, Debug)]
pub struct FeedCommand {
    #[command(subcommand)]
    command: FeedSubcommand,
}

#[derive(Subcommand, Debug)]
enum FeedSubcommand {
    Import(ImportCommand),
    Export(ExportCommand),
    Subscribe(SubscribeCommand),
    Unsubscribe(UnsubscribeCommand),
}

impl FeedCommand {
    pub async fn run(self, config: &ConfigResolver) -> anyhow::Result<()> {
        match self.command {
            FeedSubcommand::Import(import) => import.run(config).await,
            FeedSubcommand::Export(export) => export.run(config).await,
            FeedSubcommand::Subscribe(subscribe) => subscribe.run(config).await,
            FeedSubcommand::Unsubscribe(unsubscribe) => unsubscribe.run(config).await,
        }
    }
}

/// Subscribe to a feed
#[derive(Args, Debug)]
struct SubscribeCommand {
    /// Feed URL
    #[arg(long)]
    url: String,
    /// Feed category
    #[arg(long)]
    category: Option<String>,
    /// Feed requirement: must, should, or may
    #[arg(long)]
    requirement: Option<String>,
}

impl SubscribeCommand {
    async fn run(self, config: &ConfigResolver) -> anyhow::Result<()> {
        let input = self.input()?;
        let url = input.url.clone();
        let response = FeedRuntime::new(config)?
            .with_session(async |client| client.subscribe_feed(input).await)
            .await?;
        let mut stdout = io::stdout().lock();
        match response.disposition {
            SubscribeDisposition::Subscribed => writeln!(stdout, "{url} subscription recorded."),
            SubscribeDisposition::Changed => writeln!(stdout, "{url} subscription updated."),
            SubscribeDisposition::Other(disposition) => {
                writeln!(stdout, "{url} subscription updated ({disposition}).")
            }
        }?;
        Ok(())
    }

    fn input(self) -> anyhow::Result<SubscribeFeedInput> {
        let Self {
            url,
            category,
            requirement,
        } = self;
        let url = FeedUrl::parse(&url)?;
        let category: Option<Category<'static>> = category.map(Category::new).transpose()?;
        let requirement = requirement
            .as_deref()
            .map(|value| value.parse::<Requirement>().map_err(anyhow::Error::msg))
            .transpose()?;

        Ok(SubscribeFeedInput {
            url,
            requirement,
            category,
            crawl_policy: None,
        })
    }
}

/// Unsubscribe from a feed
#[derive(Args, Debug)]
struct UnsubscribeCommand {
    /// Feed URL
    #[arg(long)]
    url: String,
}

impl UnsubscribeCommand {
    async fn run(self, config: &ConfigResolver) -> anyhow::Result<()> {
        let url = FeedUrl::parse(&self.url)?;
        let response = FeedRuntime::new(config)?
            .with_session(async |client| client.unsubscribe_feed(url).await)
            .await?;
        let url = response.url;
        let mut stdout = io::stdout().lock();
        match response.disposition {
            UnsubscribeDisposition::Unsubscribed => writeln!(stdout, "{url} subscription removed."),
            UnsubscribeDisposition::Other(disposition) => {
                writeln!(stdout, "{url} subscription removed ({disposition}).")
            }
        }?;
        Ok(())
    }
}
