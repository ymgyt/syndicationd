use std::io;

use clap::Args;
use schemars::JsonSchema;
use serde::Serialize;
use synd_client::{Client, SyndApiError, payload::PageInfo};
use synd_term::types::ExportedFeed;

use crate::{config::ConfigResolver, runtime::FeedRuntime};

use super::print_json_schema;

#[derive(Serialize, JsonSchema)]
struct Export {
    feeds: Vec<ExportedFeed>,
}

/// Export subscribed feeds
#[derive(Args, Debug)]
pub struct ExportCommand {
    /// Print exported data json schema
    #[arg(
        long,
        default_value_t = false,
        action = clap::ArgAction::SetTrue,
        visible_alias = "print-json-schema",
    )]
    print_schema: bool,
}

impl ExportCommand {
    pub async fn run(self, config: &ConfigResolver) -> anyhow::Result<()> {
        if self.print_schema {
            return print_json_schema::<Export>();
        }
        let export = FeedRuntime::new(config)?
            .with_session(Export::fetch)
            .await?;
        serde_json::to_writer_pretty(io::stdout().lock(), &export)?;
        Ok(())
    }
}

impl Export {
    async fn fetch(client: &Client) -> Result<Self, SyndApiError> {
        let mut feeds = Vec::new();
        let mut after = None;
        loop {
            let page = client.fetch_subscription(after, Some(50)).await?.feeds;
            feeds.extend(page.nodes.into_iter().map(ExportedFeed::from));
            match page.page_info {
                PageInfo::Complete { .. } => break,
                PageInfo::More { next_cursor } => after = Some(next_cursor),
            }
        }
        Ok(Self { feeds })
    }
}
