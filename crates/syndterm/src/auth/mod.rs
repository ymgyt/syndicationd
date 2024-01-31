use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::config;

pub mod device_flow;
pub mod github;

#[derive(Debug, Clone, Copy)]
pub enum AuthenticationProvider {
    Github,
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Credential {
    Github { access_token: String },
}

pub fn persist_credential(cred: Credential) -> anyhow::Result<()> {
    let cred_path = cred_file();
    if let Some(parent) = cred_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut cred_file = std::fs::File::create(&cred_path)?;

    debug!(path = ?cred_path.display(), "Create credential cache file");

    serde_json::to_writer(&mut cred_file, &cred)?;

    Ok(())
}

fn cred_file() -> PathBuf {
    config::cache_dir().join("credential.json")
}

pub fn credential_from_cache() -> Option<Credential> {
    std::fs::File::open(cred_file())
        .ok()
        .and_then(|f| serde_json::from_reader(f).ok())
}
