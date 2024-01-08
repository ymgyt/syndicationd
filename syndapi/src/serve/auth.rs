use axum::{
    extract::{Request, State},
    http::{self, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::warn;

use crate::{
    client::github::GithubClient,
    principal::{Principal, User},
};

#[derive(Clone)]
pub struct Authenticator {
    github: GithubClient,
}

impl Authenticator {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            github: GithubClient::new()?,
        })
    }

    /// Authenticate from given token
    async fn authenticate(&self, token: &str) -> Result<Principal, ()> {
        let mut split = token.splitn(2, ' ');
        match (split.next(), split.next()) {
            (Some("github"), Some(access_token)) => {
                // TODO: configure cache to reduce api call

                match self.github.authenticate(access_token).await {
                    Ok(email) => Ok(Principal::User(User::from_email(email))),
                    Err(err) => {
                        warn!("Failed to authenticate github {err}");
                        Err(())
                    }
                }
            }
            (Some("me"), None) => Ok(Principal::User(User::from_email("me@ymgyt.io"))),
            _ => Err(()),
        }
    }
}

/// Check authorization header and inject Authentication
pub async fn authenticate(
    State(authenticator): State<Authenticator>,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    let Some(token) = header else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let principal = match authenticator.authenticate(token).await {
        Ok(principal) => principal,
        Err(_) => {
            warn!("Invalid token");
            return Err(StatusCode::UNAUTHORIZED);
        }
    };

    req.extensions_mut().insert(principal);

    Ok(next.run(req).await)
}
