use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::config;

pub mod device_flow;
pub mod github;

#[derive(Serialize, Deserialize, Clone)]
pub enum Authentication {
    Github { access_token: String },
}

pub fn persist_authentication(auth: Authentication) -> anyhow::Result<()> {
    let auth_path = auth_file();
    if let Some(parent) = auth_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut auth_file = std::fs::File::create(&auth_path)?;

    debug!(path = ?auth_path.display(), "Create auth cache file");

    serde_json::to_writer(&mut auth_file, &auth)?;

    Ok(())
}

fn auth_file() -> PathBuf {
    config::cache_dir().join("auth.json")
}

pub fn authenticate_from_cache() -> Option<Authentication> {
    std::fs::File::open(auth_file())
        .ok()
        .and_then(|f| serde_json::from_reader(f).ok())
}
