use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use super::{config::LocalApiConfig, token::LocalApiToken};

#[derive(Debug)]
pub(super) struct LocalApiLaunch {
    sqlite_db: PathBuf,
    timeout: Duration,
    token: LocalApiToken,
    required_dirs: Vec<PathBuf>,
}

impl LocalApiLaunch {
    pub(super) fn from_config(config: LocalApiConfig, token: LocalApiToken) -> Self {
        let required_dirs = config
            .sqlite_db
            .parent()
            .map(|parent| vec![parent.to_path_buf()])
            .unwrap_or_default();

        Self {
            sqlite_db: config.sqlite_db,
            timeout: config.timeout,
            token,
            required_dirs,
        }
    }

    pub(super) fn sqlite_db(&self) -> &Path {
        &self.sqlite_db
    }

    pub(super) fn timeout(&self) -> Duration {
        self.timeout
    }

    pub(super) fn required_dirs(&self) -> &[PathBuf] {
        &self.required_dirs
    }

    pub(super) fn into_token(self) -> LocalApiToken {
        self.token
    }
}
