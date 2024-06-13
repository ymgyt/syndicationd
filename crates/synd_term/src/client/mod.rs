use std::{fmt::Debug, time::Duration};

use anyhow::anyhow;
use graphql_client::{GraphQLQuery, Response};
use reqwest::header::{self, HeaderValue};
use serde::{de::DeserializeOwned, Serialize};
use synd_o11y::{health_check::Health, opentelemetry::extension::*};
use thiserror::Error;
use tracing::{error, Span};
use url::Url;

use crate::{
    auth::{Credential, Verified},
    client::payload::ExportSubscriptionPayload,
    config, types,
};

use self::query::subscription::SubscriptionOutput;

mod scalar;
pub use scalar::*;
#[path = "generated/mutation.rs"]
pub mod mutation;
pub mod payload;
#[path = "generated/query.rs"]
pub mod query;

#[derive(Error, Debug)]
pub enum SubscribeFeedError {
    #[error("invalid feed url: `{feed_url}` ({message})`")]
    InvalidFeedUrl { feed_url: FeedUrl, message: String },
}

#[derive(Error, Debug)]
pub enum SyndApiError {
    #[error("unauthorized")]
    Unauthorized { url: Option<Url> },
    #[error(transparent)]
    BuildRequest(#[from] reqwest::Error),
    #[error("graphql error: {errors:?}")]
    Graphql { errors: Vec<graphql_client::Error> },
    #[error(transparent)]
    SubscribeFeed(SubscribeFeedError),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// synd-api client
#[derive(Clone)]
pub struct Client {
    client: reqwest::Client,
    credential: Option<HeaderValue>,
    endpoint: Url,
}

impl Client {
    const GRAPHQL: &'static str = "/graphql";
    const HEALTH_CHECK: &'static str = "/health";

    pub fn new(endpoint: Url, timeout: Duration) -> anyhow::Result<Self> {
        let client = reqwest::ClientBuilder::new()
            .user_agent(config::client::USER_AGENT)
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(10))
            // this client specifically targets the syndicationd api, so accepts self signed certificates
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Self {
            client,
            endpoint,
            credential: None,
        })
    }

    pub(crate) fn set_credential(&mut self, cred: Verified<Credential>) {
        let mut token = HeaderValue::try_from(match cred.into_inner() {
            Credential::Github { access_token } => format!("github {access_token}"),
            Credential::Google { id_token, .. } => format!("google {id_token}"),
        })
        .unwrap();
        token.set_sensitive(true);
        self.credential = Some(token);
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch_subscription(
        &self,
        after: Option<String>,
        first: Option<i64>,
    ) -> Result<SubscriptionOutput, SyndApiError> {
        let var = query::subscription::Variables { after, first };
        let request = query::Subscription::build_query(var);
        let response: query::subscription::ResponseData = self.request(&request).await?;
        Ok(response.output)
    }

    #[tracing::instrument(skip(self))]
    pub async fn subscribe_feed(
        &self,
        input: mutation::subscribe_feed::SubscribeFeedInput,
    ) -> Result<types::Feed, SyndApiError> {
        use crate::client::mutation::subscribe_feed::ResponseCode;
        let url = input.url.clone();
        let var = mutation::subscribe_feed::Variables {
            subscribe_input: input,
        };
        let request = mutation::SubscribeFeed::build_query(var);
        let response: mutation::subscribe_feed::ResponseData = self.request(&request).await?;

        match response.subscribe_feed {
            mutation::subscribe_feed::SubscribeFeedSubscribeFeed::SubscribeFeedSuccess(success) => {
                Ok(types::Feed::from(success.feed))
            }
            mutation::subscribe_feed::SubscribeFeedSubscribeFeed::SubscribeFeedError(err) => {
                match err.status.code {
                    ResponseCode::OK => unreachable!(),
                    ResponseCode::INVALID_FEED_URL => Err(SyndApiError::SubscribeFeed(
                        SubscribeFeedError::InvalidFeedUrl {
                            feed_url: url,
                            message: err.message,
                        },
                    )),
                    err_code => Err(SyndApiError::Internal(anyhow::anyhow!(
                        "Unexpected subscribe_feed error code: {err_code:?}"
                    ))),
                }
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn unsubscribe_feed(&self, url: FeedUrl) -> Result<(), SyndApiError> {
        let var = mutation::unsubscribe_feed::Variables {
            unsubscribe_input: mutation::unsubscribe_feed::UnsubscribeFeedInput { url },
        };
        let request = mutation::UnsubscribeFeed::build_query(var);
        let response: mutation::unsubscribe_feed::ResponseData = self.request(&request).await?;

        match response.unsubscribe_feed {
            mutation::unsubscribe_feed::UnsubscribeFeedUnsubscribeFeed::UnsubscribeFeedSuccess(
                _,
            ) => Ok(()),
            mutation::unsubscribe_feed::UnsubscribeFeedUnsubscribeFeed::UnsubscribeFeedError(
                err,
            ) => Err(SyndApiError::Internal(anyhow!(
                "Failed to mutate unsubscribe_feed {err:?}"
            ))),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch_entries(
        &self,
        after: Option<String>,
        first: i64,
    ) -> Result<payload::FetchEntriesPayload, SyndApiError> {
        tracing::debug!("Fetch entries...");

        let var = query::entries::Variables { after, first };
        let request = query::Entries::build_query(var);
        let response: query::entries::ResponseData = self.request(&request).await?;

        tracing::debug!("Got response");

        Ok(response.output.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn export_subscription(
        &self,
        after: Option<String>,
        first: i64,
    ) -> anyhow::Result<ExportSubscriptionPayload> {
        let var = query::export_subscription::Variables { after, first };
        let request = query::ExportSubscription::build_query(var);
        let response: query::export_subscription::ResponseData = self.request(&request).await?;

        Ok(response.output.into())
    }

    #[tracing::instrument(skip_all, err(Display))]
    async fn request<Body, ResponseData>(&self, body: &Body) -> Result<ResponseData, SyndApiError>
    where
        Body: Serialize + Debug + ?Sized,
        ResponseData: DeserializeOwned + Debug,
    {
        let mut request = self
            .client
            .post(self.endpoint.join(Self::GRAPHQL).unwrap())
            .header(
                header::AUTHORIZATION,
                self.credential
                    .as_ref()
                    .expect("Credential not configured. this is a BUG")
                    .clone(),
            )
            .json(body)
            .build()
            .map_err(SyndApiError::BuildRequest)?;

        synd_o11y::opentelemetry::http::inject_with_baggage(
            &Span::current().context(),
            request.headers_mut(),
            std::iter::once(synd_o11y::request_id_key_value()),
        );

        tracing::debug!(url = request.url().as_str(), "Send request");

        let response: Response<ResponseData> = self
            .client
            .execute(request)
            .await?
            .error_for_status()
            .map_err(|err| match err.status().map(|s| s.as_u16()) {
                Some(401) => SyndApiError::Unauthorized {
                    url: err.url().cloned(),
                },
                _ => SyndApiError::Internal(anyhow::Error::from(err)),
            })?
            .json()
            .await?;

        match (response.data, response.errors) {
            (_, Some(errors)) if !errors.is_empty() => Err(SyndApiError::Graphql { errors }),
            (Some(data), _) => Ok(data),
            _ => Err(SyndApiError::Internal(anyhow!(
                "Unexpected error. response does not contain data and errors"
            ))),
        }
    }

    // call health check api
    pub async fn health(&self) -> anyhow::Result<Health> {
        self.client
            .get(self.endpoint.join(Self::HEALTH_CHECK).unwrap())
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
            .map_err(anyhow::Error::from)
    }
}
