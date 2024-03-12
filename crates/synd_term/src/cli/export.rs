use std::time::Duration;

use anyhow::anyhow;
use clap::Args;
use schemars::JsonSchema;
use serde::Serialize;
use url::Url;

use crate::{auth, client::Client, types::ExportedFeed};

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
    #[allow(clippy::unused_self)]
    pub async fn run(self, endpoint: Url) -> i32 {
        let err = if self.print_schema {
            Self::print_json_schema()
        } else {
            self.export(endpoint).await
        };
        if let Err(err) = err {
            tracing::error!("{err:?}");
            1
        } else {
            0
        }
    }

    fn print_json_schema() -> anyhow::Result<()> {
        let schema = schemars::schema_for!(Export);
        serde_json::to_writer_pretty(std::io::stdout(), &schema).map_err(anyhow::Error::from)
    }

    async fn export(self, endpoint: Url) -> anyhow::Result<()> {
        let mut client = Client::new(endpoint, Duration::from_secs(10))?;

        let credentials = auth::credential_from_cache()
            .ok_or_else(|| anyhow!("You are not authenticated, try login in first"))?;
        client.set_credential(credentials);

        let mut after = None;
        let mut exported_feeds = Vec::new();

        loop {
            let response = client.export_subscription(after.take(), 1).await?;
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
