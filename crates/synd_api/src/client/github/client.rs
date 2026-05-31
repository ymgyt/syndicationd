use std::{fmt::Debug, time::Duration};

use graphql_client::{GraphQLQuery, Response};
use reqwest::header::{self, HeaderValue};
use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;
use tracing::instrument;

use crate::{client::github::query, config};

#[derive(Debug, Error)]
pub enum GithubClientError {
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error(transparent)]
    InvalidHeader(#[from] header::InvalidHeaderValue),
    #[error("github graphql error: {errors:?}")]
    Graphql { errors: Vec<graphql_client::Error> },
    #[error("unexpected github graphql response")]
    UnexpectedResponse,
}

#[derive(Clone)]
pub struct GithubClient {
    client: reqwest::Client,
    endpoint: Option<&'static str>,
}

impl GithubClient {
    const ENDPOINT: &'static str = "https://api.github.com/graphql";

    /// Construct `GithubClient`.
    pub fn new() -> Result<Self, GithubClientError> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(config::USER_AGENT)
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self {
            client,
            endpoint: None,
        })
    }

    #[must_use]
    pub fn with_endpoint(self, endpoint: &'static str) -> Self {
        Self {
            endpoint: Some(endpoint),
            ..self
        }
    }

    #[instrument(name = "github::authenticate", skip_all)]
    pub async fn authenticate(&self, access_token: &str) -> Result<String, GithubClientError> {
        let variables = query::authenticate::Variables {};
        let request = query::Authenticate::build_query(variables);
        let response: query::authenticate::ResponseData =
            self.request(access_token, &request).await?;

        Ok(response.viewer.email)
    }

    async fn request<Body, ResponseData>(
        &self,
        access_token: &str,
        body: &Body,
    ) -> Result<ResponseData, GithubClientError>
    where
        Body: Serialize + ?Sized,
        ResponseData: DeserializeOwned + Debug,
    {
        let mut auth_header = HeaderValue::try_from(format!("bearer {access_token}"))?;
        auth_header.set_sensitive(true);

        let res: Response<ResponseData> = self
            .client
            .post(self.endpoint.unwrap_or(Self::ENDPOINT))
            .header(header::AUTHORIZATION, auth_header)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        match (res.data, res.errors) {
            (_, Some(errs)) if !errs.is_empty() => Err(GithubClientError::Graphql { errors: errs }),
            (Some(data), _) => Ok(data),
            _ => Err(GithubClientError::UnexpectedResponse),
        }
    }
}
