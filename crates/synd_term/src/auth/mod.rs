use std::{
    borrow::Borrow,
    cmp::Ordering,
    fmt, io,
    ops::{Deref, Sub},
    path::PathBuf,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_auth::jwt::google::JwtError;
use thiserror::Error;
use tracing::debug;

use crate::{
    application::{Cache, JwtService},
    config,
    types::Time,
};

pub mod authenticator;

#[derive(Debug, Clone, Copy)]
pub enum AuthenticationProvider {
    Github,
    Google,
}

#[derive(Debug, Error)]
pub enum CredentialError {
    #[error("google jwt expired")]
    GoogleJwtExpired { refresh_token: String },
    #[error("google jwt email not verified")]
    GoogleJwtEmailNotVerified,
    #[error("failed to open: {path} :{io_err}")]
    Open {
        #[source]
        io_err: std::io::Error,
        path: PathBuf,
    },
    #[error("serialize credential: {0}")]
    Serialize(serde_json::Error),
    #[error("deserialize credential: {0}")]
    Deserialize(serde_json::Error),
    #[error("decode jwt: {0}")]
    DecodeJwt(JwtError),
    #[error("refresh jwt id token: {0}")]
    RefreshJwt(JwtError),
    #[error("persist credential: {path} :{io_err}")]
    PersistCredential {
        #[source]
        io_err: io::Error,
        path: PathBuf,
    },
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum Credential {
    Github {
        access_token: String,
    },
    Google {
        id_token: String,
        refresh_token: String,
        expired_at: DateTime<Utc>,
    },
}

impl fmt::Debug for Credential {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credential").finish_non_exhaustive()
    }
}

/// Represents expired state
#[derive(PartialEq, Eq, Debug)]
pub(super) struct Expired<C = Credential>(pub(super) C);

/// Represents verified state
#[derive(Debug, Clone)]
pub(super) struct Verified<C = Credential>(C);

impl Deref for Verified<Credential> {
    type Target = Credential;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Borrow<Credential> for &Verified<Credential> {
    fn borrow(&self) -> &Credential {
        &self.0
    }
}

impl<C> Verified<C> {
    pub(super) fn into_inner(self) -> C {
        self.0
    }
}

/// Represents unverified state
#[derive(PartialEq, Eq, Debug)]
pub struct Unverified<C = Credential>(C);

impl From<Credential> for Unverified<Credential> {
    fn from(cred: Credential) -> Self {
        Unverified(cred)
    }
}

pub(super) enum VerifyResult {
    Verified(Verified<Credential>),
    Expired(Expired<Credential>),
}

impl Unverified<Credential> {
    pub(super) fn verify(
        self,
        jwt_service: &JwtService,
        now: DateTime<Utc>,
    ) -> Result<VerifyResult, CredentialError> {
        let credential = self.0;
        match &credential {
            Credential::Github { .. } => Ok(VerifyResult::Verified(Verified(credential))),
            Credential::Google { id_token, .. } => {
                let claims = jwt_service
                    .google
                    .decode_id_token_insecure(id_token, false)
                    .map_err(CredentialError::DecodeJwt)?;
                if !claims.email_verified {
                    return Err(CredentialError::GoogleJwtEmailNotVerified);
                }
                match claims
                    .expired_at()
                    .sub(config::credential::EXPIRE_MARGIN)
                    .cmp(&now)
                {
                    // expired
                    Ordering::Less | Ordering::Equal => {
                        debug!("Google jwt expired, trying to refresh");

                        Ok(VerifyResult::Expired(Expired(credential)))
                    }
                    // not expired
                    Ordering::Greater => Ok(VerifyResult::Verified(Verified(credential))),
                }
            }
        }
    }
}

/// Process for restoring credential from cache
pub(crate) struct Restore<'a> {
    pub(crate) jwt_service: &'a JwtService,
    pub(crate) cache: &'a Cache,
    pub(crate) now: Time,
    pub(crate) persist_when_refreshed: bool,
}

impl<'a> Restore<'a> {
    pub(crate) async fn restore(self) -> Result<Verified<Credential>, CredentialError> {
        let Restore {
            jwt_service,
            cache,
            now,
            persist_when_refreshed,
        } = self;
        let cred = cache.load_credential()?;

        match cred.verify(jwt_service, now)? {
            VerifyResult::Verified(cred) => Ok(cred),
            VerifyResult::Expired(Expired(Credential::Google { refresh_token, .. })) => {
                let cred = jwt_service.refresh_google_id_token(&refresh_token).await?;

                if persist_when_refreshed {
                    cache.persist_credential(&cred)?;
                }

                Ok(cred)
            }
            VerifyResult::Expired(_) => panic!("Unexpected verify result. this is bug"),
        }
    }
}
