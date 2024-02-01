use std::io::ErrorKind;

use anyhow::Context;
use clap::Args;

use crate::config;

/// Clear cache, log
#[derive(Args, Debug)]
pub struct ClearCommand {}

impl ClearCommand {
    #[allow(clippy::unused_self)]
    pub fn run(self) {
        let exit_code = if let Err(err) = Self::clear() {
            tracing::error!("{err}");
            1
        } else {
            0
        };

        std::process::exit(exit_code);
    }

    fn clear() -> anyhow::Result<()> {
        // remove cache
        let cache_dir = config::cache_dir();
        match std::fs::remove_dir_all(cache_dir) {
            Ok(()) => {
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
            Ok(()) => {
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
