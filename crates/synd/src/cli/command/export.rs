use std::process::ExitCode;

use clap::Args;
use schemars::JsonSchema;
use serde::Serialize;
use synd_term::types::ExportedFeed;
use tracing::error;

use crate::{cli::port::PortContext, config::ConfigResolver};

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
    pub async fn run(self, config: ConfigResolver) -> ExitCode {
        let err = if self.print_schema {
            Self::print_json_schema()
        } else {
            self.export(config).await
        };
        if let Err(err) = err {
            error!("{err:?}");
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }

    fn print_json_schema() -> anyhow::Result<()> {
        let schema = schemars::schema_for!(Export);
        serde_json::to_writer_pretty(std::io::stdout(), &schema).map_err(anyhow::Error::from)
    }

    async fn export(self, config: ConfigResolver) -> anyhow::Result<()> {
        let cx = PortContext::new(&config).await?;

        let mut after = None;
        let mut exported_feeds = Vec::new();

        loop {
            let response = cx.client.fetch_subscription(after.take(), Some(50)).await?;
            let page_info = response.feeds.page_info;
            exported_feeds.extend(response.feeds.nodes.into_iter().map(ExportedFeed::from));

            if !page_info.has_next_page {
                break;
            }
            after = page_info.end_cursor;
        }

        let output = Export {
            feeds: exported_feeds,
        };

        serde_json::to_writer_pretty(std::io::stdout(), &output)?;

        Ok(())
    }
}
