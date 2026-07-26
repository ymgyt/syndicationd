use std::process::ExitCode;

use clap::Args;
use schemars::JsonSchema;
use serde::Serialize;
use synd_client::payload::PageInfo;
use synd_term::types::ExportedFeed;

use crate::{
    cli::{command::CommandFailure, port::PortContext},
    config::ConfigResolver,
};

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
            CommandFailure::report(err)
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
        let result = async {
            let mut after = None;
            let mut exported_feeds = Vec::new();

            loop {
                let response = cx.client.fetch_subscription(after.take(), Some(50)).await?;
                let page_info = response.feeds.page_info;
                exported_feeds.extend(response.feeds.nodes.into_iter().map(ExportedFeed::from));

                match page_info {
                    PageInfo::Complete { .. } => break,
                    PageInfo::More { next_cursor } => after = Some(next_cursor),
                }
            }

            let output = Export {
                feeds: exported_feeds,
            };

            serde_json::to_writer_pretty(std::io::stdout(), &output)?;

            Ok(())
        }
        .await;

        cx.finish(result).await
    }
}
