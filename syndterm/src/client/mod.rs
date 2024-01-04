use std::{fmt::Debug, time::Duration};

use graphql_client::{GraphQLQuery, Response};
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;
use url::Url;

use crate::{auth::Authentication, config};

use self::query::user::UserSubscription;

pub mod query;

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    endpoint: Url,
}

impl Client {
    pub fn new(endpoint: Url, auth: Authentication) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();

        let mut token = HeaderValue::try_from(match auth {
            Authentication::Github { access_token } => format!("github {access_token}"),
        })?;
        token.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, token);

        let client = reqwest::ClientBuilder::new()
            .user_agent(config::USER_AGENT)
            .default_headers(headers)
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(5))
            .build()?;

        Ok(Self { client, endpoint })
    }

    pub async fn fetch_subscription(&self) -> anyhow::Result<UserSubscription> {
        let var = query::user::Variables {};
        let req = query::User::build_query(var);
        let res: query::user::ResponseData = self.request(&req).await?;
        Ok(res.subscription)
    }

    async fn request<Body, ResponseData>(&self, body: &Body) -> anyhow::Result<ResponseData>
    where
        Body: Serialize + ?Sized,
        ResponseData: DeserializeOwned + Debug,
    {
        let res: Response<ResponseData> = self
            .client
            .post(self.endpoint.clone())
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
