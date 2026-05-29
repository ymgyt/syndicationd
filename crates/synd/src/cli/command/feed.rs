use std::process::ExitCode;

use clap::{Args, Subcommand};

use crate::config::ConfigResolver;

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
            FeedSubcommand::Subscribe(subscribe) => subscribe.run(),
            FeedSubcommand::Unsubscribe(unsubscribe) => unsubscribe.run(),
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
    fn run(self) -> ExitCode {
        let Self {
            url,
            category,
            requirement,
        } = self;
        let _ = (url, category, requirement);
        not_yet_implemented()
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
    fn run(self) -> ExitCode {
        let Self { url } = self;
        let _ = url;
        not_yet_implemented()
    }
}

fn not_yet_implemented() -> ExitCode {
    eprintln!("not_yet_implemented");
    ExitCode::from(1)
}
