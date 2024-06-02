use std::{io, path::Path, time::Duration};

use anyhow::Context;
use clap::Args;
use synd_o11y::health_check::Health;
use url::Url;

use crate::{client::Client, config};

#[derive(Copy, Clone, PartialEq, Eq, Debug, clap::ValueEnum)]
pub enum CheckFormat {
    Human,
    Json,
}

/// Check application conditions
#[derive(Args, Debug)]
pub struct CheckCommand {
    #[arg(value_enum, long, default_value_t = CheckFormat::Human)]
    pub format: CheckFormat,
}

impl CheckCommand {
    #[allow(clippy::unused_self)]
    pub async fn run(self, endpoint: Url) -> i32 {
        if let Err(err) = self.check(endpoint).await {
            tracing::error!("{err:?}");
            1
        } else {
            0
        }
    }

    async fn check(self, endpoint: Url) -> anyhow::Result<()> {
        let Self { format } = self;
        let client = Client::new(endpoint, Duration::from_secs(10))?;

        let api_health = client
            .health()
            .await
            .context("api health check")
            .inspect_err(|err| eprintln!("{err:?}"))
            .ok();

        let cache_dir = config::cache::dir();
        let log_path = config::log_path();

        match format {
            CheckFormat::Human => {
                Self::print(io::stdout(), api_health, cache_dir, log_path.as_path())?;
            }
            CheckFormat::Json => {
                let health = match api_health {
                    Some(health) => serde_json::json!(&health),
                    None => serde_json::json!("unknown"),
                };
                println!(
                    "{}",
                    serde_json::json!({
                        "api": health,
                        "cache": cache_dir.display().to_string(),
                        "log": log_path.display().to_string(),
                    })
                );
            }
        }

        Ok(())
    }

    fn print(
        mut writer: impl io::Write,
        health: Option<Health>,
        cache_dir: &Path,
        log_path: &Path,
    ) -> io::Result<()> {
        let w = &mut writer;

        writeln!(
            w,
            " Api Health: {}",
            health
                .as_ref()
                .map_or("unknown".into(), |h| h.status.to_string())
        )?;
        writeln!(
            w,
            "Api Version: {}",
            health.and_then(|h| h.version).unwrap_or("unknown".into())
        )?;

        writeln!(w, "      Cache: {}", cache_dir.display())?;
        writeln!(w, "        Log: {}", log_path.display())?;
        Ok(())
    }
}
