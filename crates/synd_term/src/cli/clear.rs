use std::io::ErrorKind;

use anyhow::Context;
use clap::Args;

use crate::config;

/// Clear cache, log
#[derive(Args, Debug)]
pub struct ClearCommand {}

impl ClearCommand {
    pub async fn run(self) {
        let exit_code = if let Err(err) = self.clear().await {
            tracing::error!("{err}");
            1
        } else {
            0
        };

        std::process::exit(exit_code);
    }

    async fn clear(self) -> anyhow::Result<()> {
        // remove cache
        let cache_dir = config::cache_dir();
        match std::fs::remove_dir_all(cache_dir) {
            Ok(_) => {
                tracing::info!("Clear {}", cache_dir.display());
            }
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {}
                _ => {
                    return Err(anyhow::Error::from(err))
                        .with_context(|| format!("path: {}", cache_dir.display()))
                }
            },
        }

        // remove log
        let log_file = config::log_path();
        match std::fs::remove_file(&log_file) {
            Ok(_) => {
                tracing::info!("Clear {}", log_file.display());
            }
            Err(err) => match err.kind() {
                ErrorKind::NotFound => {}
                _ => {
                    return Err(anyhow::Error::from(err))
                        .with_context(|| format!("path: {}", log_file.display()))
                }
            },
        }

        Ok(())
    }
}
