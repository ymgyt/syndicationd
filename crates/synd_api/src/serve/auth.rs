use std::time::Duration;

use futures_util::future::BoxFuture;
use moka::future::Cache;
use synd_auth::jwt::google::JwtService as GoogleJwtService;
use tracing::warn;

use crate::{
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
}

impl Authenticator {
    pub fn new() -> anyhow::Result<Self> {
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

    pub fn local(token: impl Into<String>) -> anyhow::Result<Self> {
        let token = token.into();
        anyhow::ensure!(!token.is_empty(), "local token must not be empty");

        Ok(Self {
            kind: AuthenticatorKind::Local { token },
        })
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
        }
    }

    /// Authenticate from given token
    #[tracing::instrument(skip_all)]
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
                    tracing::debug!("Principal cache hit");
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
                    tracing::debug!("Principal cache hit");
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
        Box::pin(async move {
            match token {
                Some(token) => Authenticator::authenticate(&this, token).await,
                None => Err(()),
            }
        })
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
}
