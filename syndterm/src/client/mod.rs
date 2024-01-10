use std::{fmt::Debug, time::Duration};

use anyhow::anyhow;
use graphql_client::{GraphQLQuery, Response};
use reqwest::header::{self, HeaderValue};
use serde::{de::DeserializeOwned, Serialize};
use tracing::error;
use url::Url;

use crate::{auth::Authentication, config};

use self::query::subscription::SubscriptionOutput;

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

    pub async fn fetch_subscription(&self) -> anyhow::Result<SubscriptionOutput> {
        let var = query::subscription::Variables {};
        let req = query::Subscription::build_query(var);
        let res: query::subscription::ResponseData = self.request(&req).await?;
        Ok(res.output)
    }

    pub async fn subscribe_feed(&self, url: String) -> anyhow::Result<String> {
        let var = mutation::subscribe_feed::Variables {
            input: mutation::subscribe_feed::SubscribeFeedInput { url },
        };
        let req = mutation::SubscribeFeed::build_query(var);
        let res: mutation::subscribe_feed::ResponseData = self.request(&req).await?;

        match res.subscribe_feed {
            mutation::subscribe_feed::SubscribeFeedSubscribeFeed::SubscribeFeedSuccess(success) => {
                Ok(success.url)
            }
            mutation::subscribe_feed::SubscribeFeedSubscribeFeed::SubscribeFeedError(err) => {
                Err(anyhow!("Failed to mutate subscribe_feed {err:?}"))
            }
        }
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
