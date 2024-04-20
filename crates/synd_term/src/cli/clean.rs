use std::io::ErrorKind;

use anyhow::Context;
use clap::Args;

use crate::config;

/// Clean cache and logs
#[derive(Args, Debug)]
pub struct CleanCommand {}

impl CleanCommand {
    #[allow(clippy::unused_self)]
    pub fn run(self) -> i32 {
        if let Err(err) = Self::clean() {
            tracing::error!("{err}");
            1
        } else {
            0
        }
    }

    fn clean() -> anyhow::Result<()> {
        // remove cache
        let cache_dir = config::cache_dir();
        match std::fs::remove_dir_all(cache_dir) {
            Ok(()) => {
                tracing::info!("Remove {}", cache_dir.display());
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
                tracing::info!("Remove {}", log_file.display());
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
