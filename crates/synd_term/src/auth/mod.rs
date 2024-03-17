use std::path::{Path, PathBuf};

use chrono::Utc;
use futures_util::TryFutureExt;
use serde::{Deserialize, Serialize};
use synd_auth::jwt::google::JwtError;
use thiserror::Error;
use tracing::debug;

use crate::{application::JwtService, config};

#[derive(Debug, Clone, Copy)]
pub enum AuthenticationProvider {
    Github,
    Google,
}

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("google jwt expired")]
    GoogleJwtExpired { refresh_token: String },
    #[error("failed to open: {0}")]
    Open(std::io::Error),
    #[error("deserialize credential: {0}")]
    Deserialize(serde_json::Error),
    #[error("decode jwt: {0}")]
    DecodeJwt(JwtError),
    #[error("refresh jwt id token: {0}")]
    RefreshJwt(JwtError),
    #[error("persist credential: {0}")]
    PersistCredential(anyhow::Error),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Credential {
    Github {
        access_token: String,
    },
    Google {
        id_token: String,
        refresh_token: String,
    },
}

impl Credential {
    async fn restore_from_path(
        path: &Path,
        jwt_service: &JwtService,
    ) -> Result<Self, CredentialError> {
        tracing::info!(
            path = path.display().to_string(),
            "Restore credential from cache"
        );
        let mut f = std::fs::File::open(path).map_err(CredentialError::Open)?;
        let credential = serde_json::from_reader(&mut f).map_err(CredentialError::Deserialize)?;

        match &credential {
            Credential::Github { .. } => Ok(credential),
            Credential::Google {
                id_token,
                refresh_token,
            } => {
                let claims = jwt_service
                    .google
                    .decode_id_token_insecure(id_token, false)
                    .map_err(CredentialError::DecodeJwt)?;
                tracing::info!("{claims:?}");
                if !claims.is_expired(Utc::now()) {
                    return Ok(credential);
                }

                tracing::info!("Google jwt expired, trying to refresh");

                let id_token = jwt_service
                    .google
                    .refresh_id_token(refresh_token)
                    .await
                    .map_err(CredentialError::RefreshJwt)?;

                let credential = Credential::Google {
                    id_token,
                    refresh_token: refresh_token.clone(),
                };

                persist_credential(&credential).map_err(CredentialError::PersistCredential)?;

                tracing::info!("Persist refreshed credential");

                Ok(credential)
            }
        }
    }
}

pub fn persist_credential(cred: &Credential) -> anyhow::Result<()> {
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

pub async fn credential_from_cache(jwt_decoders: &JwtService) -> Option<Credential> {
    Credential::restore_from_path(cred_file().as_path(), jwt_decoders)
        .inspect_err(|err| {
            tracing::error!("Restore credential from cache: {err}");
        })
        .await
        .ok()
}
