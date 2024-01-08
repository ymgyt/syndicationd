use std::{fmt::Debug, time::Duration};

use graphql_client::{GraphQLQuery, Response};
use reqwest::header::{self, HeaderValue};
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;
use url::Url;

use crate::{
    auth::Authentication,
    client::{mutation::user::SubscribeFeedInput, query::user::UserSubscription},
    config,
};

pub mod mutation;
pub mod query;

#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    credential: Option<HeaderValue>,
    endpoint: Url,
}

impl Client {
    pub fn new(endpoint: Url) -> anyhow::Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(config::USER_AGENT)
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(5))
            .build()?;

        Ok(Self {
            client,
            endpoint,
            credential: None,
        })
    }

    pub fn set_credential(&mut self, auth: Authentication) {
        let mut token = HeaderValue::try_from(match auth {
            Authentication::Github { access_token } => format!("github {access_token}"),
        })
        .unwrap();
        token.set_sensitive(true);
        self.credential = Some(token);
    }

    pub async fn fetch_subscription(&self) -> anyhow::Result<UserSubscription> {
        let var = query::user::Variables {};
        let req = query::User::build_query(var);
        let res: query::user::ResponseData = self.request(&req).await?;
        Ok(res.subscription)
    }

    pub async fn subscribe_feed(&self, url: String) -> anyhow::Result<()> {
        let var = mutation::user::Variables {
            input: SubscribeFeedInput { url },
        };
        let req = mutation::User::build_query(var);
        let _res: mutation::user::ResponseData = self.request(&req).await?;
        Ok(())
    }

    async fn request<Body, ResponseData>(&self, body: &Body) -> anyhow::Result<ResponseData>
    where
        Body: Serialize + ?Sized,
        ResponseData: DeserializeOwned + Debug,
    {
        let res: Response<ResponseData> = self
            .client
            .post(self.endpoint.clone())
            .header(
                header::AUTHORIZATION,
                self.credential
                    .as_ref()
                    .expect("Credential not configured. this is BUG")
                    .clone(),
            )
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
                Err(anyhow::anyhow!("failed to request synd api"))
            }
            (Some(data), _) => Ok(data),
            _ => Err(anyhow::anyhow!("unexpected response",)),
        }
    }
}
