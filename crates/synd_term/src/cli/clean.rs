use std::{io::ErrorKind, path::PathBuf};

use anyhow::Context;
use clap::Args;

use crate::{application::Cache, config};

/// Clean cache and logs
#[derive(Args, Debug)]
pub struct CleanCommand {
    /// Cache directory
    #[arg(
        long,
        default_value = config::cache::dir().to_path_buf().into_os_string(),
    )]
    cache_dir: PathBuf,
}

impl CleanCommand {
    #[allow(clippy::unused_self)]
    pub fn run(self) -> i32 {
        if let Err(err) = self.clean() {
            tracing::error!("{err}");
            1
        } else {
            0
        }
    }

    fn clean(self) -> anyhow::Result<()> {
        let CleanCommand { cache_dir } = self;

        let cache = Cache::new(&cache_dir);
        cache
            .clean()
            .map_err(anyhow::Error::from)
            .with_context(|| format!("path: {}", cache_dir.display()))?;

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
