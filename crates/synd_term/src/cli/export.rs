use std::time::Duration;

use anyhow::anyhow;
use clap::Args;
use url::Url;

use crate::{auth, client::Client};

/// Export subscribed feeds
#[derive(Args, Debug)]
pub struct ExportCommand {}

impl ExportCommand {
    #[allow(clippy::unused_self)]
    pub async fn run(self, endpoint: Url) -> i32 {
        if let Err(err) = self.export(endpoint).await {
            tracing::error!("{err:?}");
            1
        } else {
            0
        }
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

        let output = serde_json::json! {{
            "feeds": exported_feeds,
        }};

        serde_json::to_writer_pretty(std::io::stdout(), &output)?;

        Ok(())
    }
}
