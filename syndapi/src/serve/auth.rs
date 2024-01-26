use std::time::Duration;

use moka::future::Cache;
use tracing::warn;

use crate::{
    client::github::GithubClient,
    principal::{Principal, User},
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

    /// Authenticate from given token
    pub async fn authenticate(&self, token: impl AsRef<str>) -> Result<Principal, ()> {
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
                        warn!("Failed to authenticate github {err}");
                        Err(())
                    }
                }
            }
            _ => Err(()),
        }
    }
}
