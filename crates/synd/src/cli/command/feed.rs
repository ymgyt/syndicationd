use std::process::ExitCode;

use clap::{Args, Subcommand};
use synd_client::payload::{SubscribeDisposition, SubscribeFeedInput};
use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::{
    cli::{command::CommandFailure, port::PortContext},
    config::ConfigResolver,
};

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
    pub async fn run(self, config: ConfigResolver) -> ExitCode {
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
    async fn run(self, config: ConfigResolver) -> ExitCode {
        match self.subscribe(config).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => CommandFailure::report(err),
        }
    }

    async fn subscribe(self, config: ConfigResolver) -> anyhow::Result<()> {
        let input = self.input()?;
        let url = input.url.clone();
        let cx = PortContext::new(&config).await?;
        let result = async {
            let response = cx.client.subscribe_feed(input).await?;
            match response.disposition {
                SubscribeDisposition::Subscribed => {
                    println!("{url} subscription recorded.");
                }
                SubscribeDisposition::Changed => {
                    println!("{url} subscription updated.");
                }
                SubscribeDisposition::Other(disposition) => {
                    println!("{url} subscription updated ({disposition}).");
                }
            }
            Ok(())
        }
        .await;

        cx.finish(result).await
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
    async fn run(self, config: ConfigResolver) -> ExitCode {
        match self.unsubscribe(config).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => CommandFailure::report(err),
        }
    }

    async fn unsubscribe(self, config: ConfigResolver) -> anyhow::Result<()> {
        let url = FeedUrl::parse(&self.url)?;
        let cx = PortContext::new(&config).await?;
        let result = async {
            cx.client.unsubscribe_feed(url.clone()).await?;
            println!("{url} subscription removed.");
            Ok(())
        }
        .await;

        cx.finish(result).await
    }
}
