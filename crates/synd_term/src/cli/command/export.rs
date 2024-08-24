use std::{path::PathBuf, process::ExitCode};

use clap::Args;
use schemars::JsonSchema;
use serde::Serialize;
use url::Url;

use crate::{cli::port::PortContext, config, types::ExportedFeed};

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
    /// Cache directory
    #[arg(
        long,
        default_value = config::cache::dir().to_path_buf().into_os_string(),
    )]
    cache_dir: PathBuf,
}

impl ExportCommand {
    pub async fn run(self, endpoint: Url) -> ExitCode {
        let err = if self.print_schema {
            Self::print_json_schema()
        } else {
            self.export(endpoint).await
        };
        if let Err(err) = err {
            tracing::error!("{err:?}");
            ExitCode::from(1)
        } else {
            ExitCode::SUCCESS
        }
    }

    fn print_json_schema() -> anyhow::Result<()> {
        let schema = schemars::schema_for!(Export);
        serde_json::to_writer_pretty(std::io::stdout(), &schema).map_err(anyhow::Error::from)
    }

    async fn export(self, endpoint: Url) -> anyhow::Result<()> {
        let cx = PortContext::new(endpoint, self.cache_dir).await?;

        let mut after = None;
        let mut exported_feeds = Vec::new();

        loop {
            let response = cx.client.export_subscription(after.take(), 50).await?;
            exported_feeds.extend(response.feeds);

            if !response.page_info.has_next_page {
                break;
            }
            after = response.page_info.end_cursor;
        }

        let output = Export {
            feeds: exported_feeds,
        };

        serde_json::to_writer_pretty(std::io::stdout(), &output)?;

        Ok(())
    }
}
