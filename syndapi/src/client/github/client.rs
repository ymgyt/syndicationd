use std::{fmt::Debug, time::Duration};

use graphql_client::{GraphQLQuery, Response};
use reqwest::header::{self, HeaderValue};
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;

use crate::{client::github::query, config};

#[derive(Clone)]
pub struct GithubClient {
    client: reqwest::Client,
}

impl GithubClient {
    const ENDPOINT: &'static str = "https://api.github.com/graphql";

    /// Construct GithubClient.
    pub fn new() -> anyhow::Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(config::USER_AGENT)
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self { client })
    }

    pub async fn authenticate(&self, access_token: &str) -> anyhow::Result<String> {
        let variables = query::authenticate::Variables {};
        let req = query::Authenticate::build_query(variables);
        let res: query::authenticate::ResponseData = self.request(access_token, &req).await?;

        Ok(res.viewer.email)
    }

    async fn request<Body, ResponseData>(
        &self,
        access_token: &str,
        body: &Body,
    ) -> anyhow::Result<ResponseData>
    where
        Body: Serialize + ?Sized,
        ResponseData: DeserializeOwned + Debug,
    {
        let mut auth_header = HeaderValue::try_from(format!("bearer {}", access_token))?;
        auth_header.set_sensitive(true);

        let res: Response<ResponseData> = self
            .client
            .post(Self::ENDPOINT)
            .header(header::AUTHORIZATION, auth_header)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        match (res.data, res.errors) {
            (_, Some(errs)) if !errs.is_empty() => {
                for err in errs {
                    error!("{err:?}");
                }
                Err(anyhow::anyhow!("failed to request github api"))
            }
            (Some(data), _) => Ok(data),
            _ => Err(anyhow::anyhow!("unexpected response",)),
        }
    }
}
