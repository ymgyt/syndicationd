use std::time::Duration;

use futures_util::future::BoxFuture;
use moka::future::Cache;
use tracing::warn;

use crate::{
    client::github::GithubClient,
    principal::{Principal, User},
    serve::layer::authenticate::Authenticate,
};

#[derive(Clone)]
pub struct Authenticator {
    github: GithubClient,
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
            cache,
        })
    }

    #[must_use]
    pub fn with_client(self, github: GithubClient) -> Self {
        Self { github, ..self }
    }

    /// Authenticate from given token
    pub async fn authenticate<S>(&self, token: S) -> Result<Principal, ()>
    where
        S: AsRef<str>,
    {
        let token = token.as_ref();
        let mut split = token.splitn(2, ' ');
        match (split.next(), split.next()) {
            (Some("github"), Some(access_token)) => {
                if let Some(principal) = self.cache.get(token).await {
                    tracing::info!("Principal cache hit");
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
