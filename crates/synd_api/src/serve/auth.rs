use std::time::Duration;

use futures_util::future::BoxFuture;
use moka::future::Cache;
use synd_auth::jwt::google::JwtService as GoogleJwtService;
use tracing::warn;
use tracing::{debug, instrument};

use crate::{
    Error, Result as ApiResult,
    client::github::GithubClient,
    principal::{Principal, User},
    serve::layer::authenticate::Authenticate,
};

#[derive(Clone)]
pub struct Authenticator {
    kind: AuthenticatorKind,
}

#[derive(Clone)]
enum AuthenticatorKind {
    Remote {
        github: GithubClient,
        google: Box<GoogleJwtService>,
        cache: Cache<String, Principal>,
    },
    Local {
        token: String,
    },
    TrustedLocal,
}

impl Authenticator {
    pub fn new() -> ApiResult<Self> {
        let cache = Cache::builder()
            .max_capacity(1024 * 1024)
            .time_to_live(Duration::from_hours(1))
            .build();

        Ok(Self {
            kind: AuthenticatorKind::Remote {
                github: GithubClient::new()?,
                google: Box::default(),
                cache,
            },
        })
    }

    pub fn local(token: impl Into<String>) -> ApiResult<Self> {
        let token = token.into();
        if token.is_empty() {
            return Err(Error::EmptyLocalToken);
        }

        Ok(Self {
            kind: AuthenticatorKind::Local { token },
        })
    }

    pub fn trusted_local() -> Self {
        Self {
            kind: AuthenticatorKind::TrustedLocal,
        }
    }

    #[must_use]
    pub fn with_github_client(self, github: GithubClient) -> Self {
        match self.kind {
            AuthenticatorKind::Remote { google, cache, .. } => Self {
                kind: AuthenticatorKind::Remote {
                    github,
                    google,
                    cache,
                },
            },
            AuthenticatorKind::Local { token } => Self {
                kind: AuthenticatorKind::Local { token },
            },
            AuthenticatorKind::TrustedLocal => Self {
                kind: AuthenticatorKind::TrustedLocal,
            },
        }
    }

    #[must_use]
    pub fn with_google_jwt(self, google: GoogleJwtService) -> Self {
        match self.kind {
            AuthenticatorKind::Remote { github, cache, .. } => Self {
                kind: AuthenticatorKind::Remote {
                    github,
                    google: Box::new(google),
                    cache,
                },
            },
            AuthenticatorKind::Local { token } => Self {
                kind: AuthenticatorKind::Local { token },
            },
            AuthenticatorKind::TrustedLocal => Self {
                kind: AuthenticatorKind::TrustedLocal,
            },
        }
    }

    /// Authenticate from given token
    #[instrument(skip_all)]
    pub async fn authenticate<S>(&self, token: S) -> Result<Principal, ()>
    where
        S: AsRef<str>,
    {
        let token = token.as_ref();
        match &self.kind {
            AuthenticatorKind::Remote {
                github,
                google,
                cache,
            } => Self::authenticate_remote(github, google, cache, token).await,
            AuthenticatorKind::Local {
                token: expected_token,
            } => Self::authenticate_local(expected_token, token),
            AuthenticatorKind::TrustedLocal => Ok(Principal::User(User::local())),
        }
    }

    async fn authenticate_optional(&self, token: Option<String>) -> Result<Principal, ()> {
        match token {
            Some(token) => self.authenticate(token).await,
            None if matches!(self.kind, AuthenticatorKind::TrustedLocal) => {
                Ok(Principal::User(User::local()))
            }
            None => Err(()),
        }
    }

    async fn authenticate_remote(
        github: &GithubClient,
        google: &GoogleJwtService,
        cache: &Cache<String, Principal>,
        token: &str,
    ) -> Result<Principal, ()> {
        let mut split = token.splitn(2, ' ');
        match (split.next(), split.next()) {
            (Some("github"), Some(access_token)) => {
                if let Some(principal) = cache.get(token).await {
                    debug!("Principal cache hit");
                    return Ok(principal);
                }

                match github.authenticate(access_token).await {
                    Ok(email) => {
                        let principal = Principal::User(User::from_email(email));

                        cache.insert(token.to_owned(), principal.clone()).await;

                        Ok(principal)
                    }
                    Err(err) => {
                        warn!("Failed to authenticate github: {err}");
                        Err(())
                    }
                }
            }
            (Some("google"), Some(id_token)) => {
                if let Some(principal) = cache.get(id_token).await {
                    debug!("Principal cache hit");
                    return Ok(principal);
                }

                match google.decode_id_token(id_token).await {
                    Ok(claims) => {
                        if !claims.email_verified {
                            warn!("Google jwt claims email is not verified");
                            return Err(());
                        }
                        let principal = Principal::User(User::from_email(claims.email));

                        cache.insert(id_token.to_owned(), principal.clone()).await;

                        Ok(principal)
                    }
                    Err(err) => {
                        // If a lot of intentional invalid id tokens are sent
                        // google's api limit will be exceeded.
                        // To prevent this, it is necessary to cache the currently valid kids
                        // and discard jwt headers with other kids.
                        warn!("Failed to authenticate google: {err}");
                        Err(())
                    }
                }
            }
            _ => Err(()),
        }
    }

    fn authenticate_local(expected_token: &str, token: &str) -> Result<Principal, ()> {
        let mut split = token.splitn(2, ' ');
        match (split.next(), split.next()) {
            (Some(scheme), Some(actual_token))
                if scheme.eq_ignore_ascii_case("Bearer") && actual_token == expected_token =>
            {
                Ok(Principal::User(User::local()))
            }
            _ => Err(()),
        }
    }
}

impl Authenticate for Authenticator {
    type Output = BoxFuture<'static, Result<Principal, ()>>;

    fn authenticate(&self, token: Option<String>) -> Self::Output {
        let this = self.clone();
        Box::pin(async move { this.authenticate_optional(token).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn local_auth_accepts_matching_bearer_token() {
        let authenticator = Authenticator::local("secret").unwrap();
        let principal = authenticator.authenticate("Bearer secret").await.unwrap();

        assert_eq!(principal.principal_id(), "local");
    }

    #[tokio::test]
    async fn local_auth_rejects_missing_or_wrong_token() {
        let authenticator = Authenticator::local("secret").unwrap();

        assert!(authenticator.authenticate("Bearer wrong").await.is_err());
        assert!(authenticator.authenticate("github secret").await.is_err());
        assert!(authenticator.authenticate("").await.is_err());
    }

    #[test]
    fn local_auth_requires_non_empty_token() {
        assert!(Authenticator::local("").is_err());
    }

    #[tokio::test]
    async fn trusted_local_auth_accepts_missing_token() {
        let authenticator = Authenticator::trusted_local();
        let principal = authenticator.authenticate_optional(None).await.unwrap();

        assert_eq!(principal.principal_id(), "local");
    }
}
