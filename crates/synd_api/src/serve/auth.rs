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
    github: GithubClient,
    google: GoogleJwtService,
    cache: Cache<String, Principal>,
}

impl Authenticator {
    pub fn new() -> anyhow::Result<Self> {
        let cache = Cache::builder()
            .max_capacity(1024 * 1024)
            .time_to_live(Duration::from_secs(60 * 60))
            .build();

        Ok(Self {
            github: GithubClient::new()?,
            google: GoogleJwtService::default(),
            cache,
        })
    }

    #[must_use]
    pub fn with_client(self, github: GithubClient) -> Self {
        Self { github, ..self }
    }

    /// Authenticate from given token
    #[tracing::instrument(skip_all)]
    pub async fn authenticate<S>(&self, token: S) -> Result<Principal, ()>
    where
        S: AsRef<str>,
    {
        let token = token.as_ref();
        let mut split = token.splitn(2, ' ');
        match (split.next(), split.next()) {
            (Some("github"), Some(access_token)) => {
                if let Some(principal) = self.cache.get(token).await {
                    tracing::debug!("Principal cache hit");
                    return Ok(principal);
                }

                match self.github.authenticate(access_token).await {
                    Ok(email) => {
                        let principal = Principal::User(User::from_email(email));

                        self.cache.insert(token.to_owned(), principal.clone()).await;

                        Ok(principal)
                    }
                    Err(err) => {
                        warn!("Failed to authenticate github: {err}");
                        Err(())
                    }
                }
            }
            (Some("google"), Some(id_token)) => {
                if let Some(principal) = self.cache.get(id_token).await {
                    tracing::debug!("Principal cache hit");
                    return Ok(principal);
                }

                match self.google.decode_id_token(id_token).await {
                    Ok(claims) => {
                        if !claims.email_verified {
                            warn!("Google jwt claims email is not verified");
                            return Err(());
                        }
                        let principal = Principal::User(User::from_email(claims.email));

                        self.cache
                            .insert(id_token.to_owned(), principal.clone())
                            .await;

                        Ok(principal)
                    }
                    Err(err) => {
                        // Id a lot of intentional invalid id tokens are sent
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
