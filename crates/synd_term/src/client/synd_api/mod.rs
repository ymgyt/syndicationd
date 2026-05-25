use std::{fmt::Debug, time::Duration};

use anyhow::anyhow;
use futures_util::{SinkExt, Stream, StreamExt};
use graphql_client::Response;
use reqwest::header::{self, HeaderValue};
use serde::{Serialize, de::DeserializeOwned};
use synd_support::o11y::{health_check::Health, opentelemetry::extension::*};
use thiserror::Error;
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use tracing::Span;
use url::Url;

use crate::{
    auth::{Credential, Verified},
    client::synd_api::payload::{
        ExportSubscriptionPayload, FeedRegistryEvent, InitialFeedRegistryPayload,
        RefreshFeedPayload, RefreshStatus, SubscribeFeedInput, SubscribeFeedPayload,
        SubscriptionPayload,
    },
    config,
};

mod scalar;
pub use scalar::*;
pub mod payload;

#[derive(Error, Debug)]
pub enum SubscribeFeedError {
    #[error("invalid feed url: `{feed_url}` ({message})`")]
    InvalidFeedUrl { feed_url: FeedUrl, message: String },
    #[error("{feed_url} {message}")]
    FeedUnavailable { feed_url: FeedUrl, message: String },
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
    #[expect(clippy::struct_field_names)]
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

    pub(crate) fn set_local_token(&mut self, token: &str) -> anyhow::Result<()> {
        let mut token = HeaderValue::try_from(format!("Bearer {token}"))?;
        token.set_sensitive(true);
        self.credential = Some(token);
        Ok(())
    }

    pub(crate) fn supports_feed_registry_events(&self) -> bool {
        self.endpoint.scheme() == "http"
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch_initial_feed_registry(
        &self,
        subscriptions_first: i64,
        timeline_first: i64,
    ) -> Result<InitialFeedRegistryPayload, SyndApiError> {
        #[derive(Serialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct Variables {
            subscriptions_first: i64,
            timeline_first: i64,
        }
        #[derive(Debug, serde::Deserialize)]
        struct ResponseData {
            output: InitialFeedRegistryPayload,
        }

        let response: Response<ResponseData> = self
            .execute_graphql(&graphql(
                INITIAL_FEED_REGISTRY_QUERY,
                Variables {
                    subscriptions_first,
                    timeline_first,
                },
            ))
            .await?;

        let errors = response.errors.unwrap_or_default();
        match response.data {
            Some(data) => {
                if !errors.is_empty() {
                    tracing::warn!(
                        errors = ?errors,
                        "initial feed registry query returned partial GraphQL errors"
                    );
                }
                Ok(data.output)
            }
            None if !errors.is_empty() => Err(SyndApiError::Graphql { errors }),
            None => Err(SyndApiError::Internal(anyhow!(
                "Unexpected error. response does not contain data and errors"
            ))),
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch_subscription(
        &self,
        after: Option<String>,
        first: Option<i64>,
    ) -> Result<SubscriptionPayload, SyndApiError> {
        #[derive(Serialize, Debug)]
        struct Variables {
            after: Option<String>,
            first: Option<i64>,
        }
        #[derive(Debug, serde::Deserialize)]
        struct ResponseData {
            output: SubscriptionPayload,
        }

        let response: ResponseData = self
            .request(&graphql(SUBSCRIPTION_QUERY, Variables { after, first }))
            .await?;
        Ok(response.output)
    }

    #[tracing::instrument(skip(self))]
    pub async fn subscribe_feed(
        &self,
        input: SubscribeFeedInput,
    ) -> Result<SubscribeFeedPayload, SyndApiError> {
        let url = input.url.clone();
        #[derive(Serialize, Debug)]
        struct Variables {
            input: SubscribeFeedInput,
        }
        #[derive(Debug, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponseData {
            subscribe_feed: SubscribeFeedPayload,
        }

        let response: ResponseData = self
            .request(&graphql(SUBSCRIBE_FEED_MUTATION, Variables { input }))
            .await
            .map_err(|err| match err {
                SyndApiError::Graphql { errors } => {
                    SyndApiError::SubscribeFeed(SubscribeFeedError::FeedUnavailable {
                        feed_url: url,
                        message: format!("{errors:?}"),
                    })
                }
                err => err,
            })?;
        Ok(response.subscribe_feed)
    }

    #[tracing::instrument(skip(self))]
    pub async fn unsubscribe_feed(&self, url: FeedUrl) -> Result<(), SyndApiError> {
        #[derive(Serialize, Debug)]
        struct Variables {
            input: UnsubscribeFeedInput,
        }
        #[derive(Serialize, Debug)]
        struct UnsubscribeFeedInput {
            url: FeedUrl,
        }
        #[derive(Debug, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponseData {
            unsubscribe_feed: UnsubscribeFeedPayload,
        }
        #[derive(Debug, serde::Deserialize)]
        struct UnsubscribeFeedPayload {
            status: payload::ResponseStatus,
        }

        let response: ResponseData = self
            .request(&graphql(
                UNSUBSCRIBE_FEED_MUTATION,
                Variables {
                    input: UnsubscribeFeedInput { url },
                },
            ))
            .await?;
        let _ = response.unsubscribe_feed.status.code;
        Ok(())
    }

    pub async fn refresh_feed(&self, url: FeedUrl) -> Result<RefreshFeedPayload, SyndApiError> {
        #[derive(Serialize, Debug)]
        struct Variables {
            input: RefreshFeedInput,
        }
        #[derive(Serialize, Debug)]
        struct RefreshFeedInput {
            url: FeedUrl,
        }
        #[derive(Debug, serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct ResponseData {
            refresh_feed: RefreshFeedPayload,
        }

        let response: ResponseData = self
            .request(&graphql(
                REFRESH_FEED_MUTATION,
                Variables {
                    input: RefreshFeedInput { url },
                },
            ))
            .await?;
        Ok(response.refresh_feed)
    }

    pub async fn fetch_feed_status(&self, url: FeedUrl) -> Result<RefreshStatus, SyndApiError> {
        #[derive(Serialize, Debug)]
        struct Variables {
            url: FeedUrl,
        }

        let response: payload::FeedStatusResponseData = self
            .request(&graphql(FEED_STATUS_QUERY, Variables { url }))
            .await?;
        Ok(response.output.feed_status)
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch_entries(
        &self,
        after: Option<String>,
        first: i64,
    ) -> Result<payload::FetchEntriesPayload, SyndApiError> {
        tracing::debug!("Fetch entries...");

        #[derive(Serialize, Debug)]
        struct Variables {
            after: Option<String>,
            first: i64,
        }

        let response: payload::EntriesResponseData = self
            .request(&graphql(ENTRIES_QUERY, Variables { after, first }))
            .await?;

        tracing::debug!("Got response");

        Ok(response.output.into())
    }

    #[tracing::instrument(skip(self))]
    pub async fn export_subscription(
        &self,
        after: Option<String>,
        first: i64,
    ) -> anyhow::Result<ExportSubscriptionPayload> {
        let payload = self.fetch_subscription(after, Some(first)).await?;
        Ok(ExportSubscriptionPayload {
            feeds: payload.feeds.nodes.into_iter().map(Into::into).collect(),
            page_info: payload.feeds.page_info,
        })
    }

    #[tracing::instrument(skip_all, err(Display))]
    async fn request<Body, ResponseData>(&self, body: &Body) -> Result<ResponseData, SyndApiError>
    where
        Body: Serialize + Debug + ?Sized,
        ResponseData: DeserializeOwned + Debug,
    {
        let response: Response<ResponseData> = self.execute_graphql(body).await?;

        match (response.data, response.errors) {
            (_, Some(errors)) if !errors.is_empty() => Err(SyndApiError::Graphql { errors }),
            (Some(data), _) => Ok(data),
            _ => Err(SyndApiError::Internal(anyhow!(
                "Unexpected error. response does not contain data and errors"
            ))),
        }
    }

    async fn execute_graphql<Body, ResponseData>(
        &self,
        body: &Body,
    ) -> Result<Response<ResponseData>, SyndApiError>
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

        synd_support::o11y::opentelemetry::http::inject_with_baggage(
            &Span::current().context(),
            request.headers_mut(),
            std::iter::once(synd_support::o11y::request_id_key_value()),
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

        Ok(response)
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

    #[tracing::instrument(skip(self))]
    pub async fn next_feed_registry_event(&self) -> Result<FeedRegistryEvent, SyndApiError> {
        let mut socket = self.connect_feed_registry_event_socket().await?;
        wait_for_feed_registry_event(&mut socket).await
    }

    #[tracing::instrument(skip(self, events))]
    pub async fn run_feed_registry_events(
        &self,
        events: mpsc::UnboundedSender<FeedRegistryEvent>,
    ) -> Result<(), SyndApiError> {
        let mut socket = self.connect_feed_registry_event_socket().await?;

        loop {
            let event = wait_for_feed_registry_event(&mut socket).await?;
            if events.send(event).is_err() {
                return Ok(());
            }
        }
    }

    async fn connect_feed_registry_event_socket(
        &self,
    ) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, SyndApiError> {
        let ws_url = self.graphql_ws_endpoint()?;
        let mut request = ws_url
            .as_str()
            .into_client_request()
            .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)))?;
        request.headers_mut().insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("graphql-transport-ws"),
        );
        request.headers_mut().insert(
            header::AUTHORIZATION,
            self.credential
                .as_ref()
                .expect("Credential not configured. this is a BUG")
                .clone(),
        );

        let (mut socket, _) = connect_async(request)
            .await
            .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)))?;

        socket
            .send(Message::text(r#"{"type":"connection_init"}"#))
            .await
            .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)))?;
        wait_for_connection_ack(&mut socket).await?;

        let subscribe = serde_json::json!({
            "id": "timelineChanged",
            "type": "subscribe",
            "payload": {
                "query": TIMELINE_CHANGED_SUBSCRIPTION,
            },
        });
        socket
            .send(Message::text(subscribe.to_string()))
            .await
            .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)))?;

        Ok(socket)
    }

    fn graphql_ws_endpoint(&self) -> Result<Url, SyndApiError> {
        let mut url = self
            .endpoint
            .join("/graphql/ws")
            .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)))?;
        let scheme = match url.scheme() {
            "http" => "ws",
            "https" => {
                return Err(SyndApiError::Internal(anyhow!(
                    "GraphQL subscription over TLS is not implemented in synd-term yet"
                )));
            }
            scheme => {
                return Err(SyndApiError::Internal(anyhow!(
                    "unsupported GraphQL subscription endpoint scheme: {scheme}"
                )));
            }
        };
        url.set_scheme(scheme)
            .map_err(|()| SyndApiError::Internal(anyhow!("failed to set websocket scheme")))?;
        Ok(url)
    }
}

#[derive(Debug, Serialize)]
struct GraphqlRequest<V> {
    query: &'static str,
    variables: V,
}

fn graphql<V>(query: &'static str, variables: V) -> GraphqlRequest<V> {
    GraphqlRequest { query, variables }
}

async fn wait_for_connection_ack<S>(socket: &mut S) -> Result<(), SyndApiError>
where
    S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let value = next_ws_json(socket).await?;
        match value.get("type").and_then(serde_json::Value::as_str) {
            Some("connection_ack") => return Ok(()),
            Some("connection_error" | "error") => {
                return Err(SyndApiError::Internal(anyhow!(
                    "GraphQL subscription connection failed: {value}"
                )));
            }
            _ => {}
        }
    }
}

async fn wait_for_feed_registry_event<S>(socket: &mut S) -> Result<FeedRegistryEvent, SyndApiError>
where
    S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let value = next_ws_json(socket).await?;
        match value.get("type").and_then(serde_json::Value::as_str) {
            Some("next") => {
                let payload = value
                    .get("payload")
                    .ok_or_else(|| SyndApiError::Internal(anyhow!("missing payload")))?;
                if let Some(errors) = payload.get("errors") {
                    return Err(SyndApiError::Internal(anyhow!(
                        "GraphQL subscription error: {errors}"
                    )));
                }
                let event = payload
                    .get("data")
                    .and_then(|data| data.get("timelineChanged"))
                    .cloned()
                    .ok_or_else(|| {
                        SyndApiError::Internal(anyhow!("missing timelineChanged payload"))
                    })?;
                let event = serde_json::from_value(event)
                    .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)))?;
                return Ok(FeedRegistryEvent::TimelineChanged(event));
            }
            Some("error") => {
                return Err(SyndApiError::Internal(anyhow!(
                    "GraphQL subscription error: {value}"
                )));
            }
            Some("complete") => {
                return Err(SyndApiError::Internal(anyhow!(
                    "GraphQL subscription completed before receiving an event"
                )));
            }
            _ => {}
        }
    }
}

async fn next_ws_json<S>(socket: &mut S) -> Result<serde_json::Value, SyndApiError>
where
    S: Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let message = socket
            .next()
            .await
            .ok_or_else(|| SyndApiError::Internal(anyhow!("GraphQL subscription closed")))?
            .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)))?;
        match message {
            Message::Text(text) => {
                return serde_json::from_str(text.as_ref())
                    .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)));
            }
            Message::Binary(bytes) => {
                return serde_json::from_slice(&bytes)
                    .map_err(|err| SyndApiError::Internal(anyhow::Error::from(err)));
            }
            Message::Close(frame) => {
                return Err(SyndApiError::Internal(anyhow!(
                    "GraphQL subscription closed: {frame:?}"
                )));
            }
            Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
        }
    }
}

const INITIAL_FEED_REGISTRY_QUERY: &str = r"
query InitialFeedRegistry($subscriptionsFirst: Int!, $timelineFirst: Int!) {
  output: feedRegistry {
    subscriptions(first: $subscriptionsFirst) {
      nodes {
        url
        requirement
        category
        refreshPolicy {
          kind
          intervalSeconds
        }
        refreshStatus {
          state
          requestId
          lastAttemptAt
          lastSuccessAt
          lastFailureAt
          lastErrorMessage
        }
        feed {
          type
          title
          updated
          websiteUrl
          description
          generator
          entries(first: 10) {
            nodes {
              title
              published
              updated
              summary
            }
          }
          links {
            nodes {
              href
              rel
              mediaType
              title
            }
          }
          authors {
            nodes
          }
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
    }
    timeline {
      entries(first: $timelineFirst) {
        nodes {
          title
          published
          updated
          summary
          websiteUrl
          feed {
            title
            url
            requirement
            category
          }
        }
        pageInfo {
          hasNextPage
          endCursor
        }
      }
    }
  }
}
";

const SUBSCRIPTION_QUERY: &str = r"
query Subscription($after: String, $first: Int) {
  output: subscription {
    feeds(after: $after, first: $first) {
      nodes {
        url
        requirement
        category
        refreshPolicy {
          kind
          intervalSeconds
        }
        refreshStatus {
          state
          requestId
          lastAttemptAt
          lastSuccessAt
          lastFailureAt
          lastErrorMessage
        }
        feed {
          type
          title
          updated
          websiteUrl
          description
          generator
          entries(first: 10) {
            nodes {
              title
              published
              updated
              summary
            }
          }
          links {
            nodes {
              href
              rel
              mediaType
              title
            }
          }
          authors {
            nodes
          }
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
    }
  }
}
";

const ENTRIES_QUERY: &str = r"
query Entries($after: String, $first: Int!) {
  output: subscription {
    entries(after: $after, first: $first) {
      nodes {
        title
        published
        updated
        summary
        websiteUrl
        feed {
          title
          url
          requirement
          category
        }
      }
      pageInfo {
        hasNextPage
        endCursor
      }
    }
  }
}
";

const FEED_STATUS_QUERY: &str = r"
query FeedStatus($url: FeedUrl!) {
  output: subscription {
    feedStatus(url: $url) {
      state
      requestId
      lastAttemptAt
      lastSuccessAt
      lastFailureAt
      lastErrorMessage
    }
  }
}
";

const SUBSCRIBE_FEED_MUTATION: &str = r"
mutation SubscribeFeed($input: SubscribeFeedInput!) {
  subscribeFeed(input: $input) {
    status { code }
    url
    requestId
    disposition
  }
}
";

const UNSUBSCRIBE_FEED_MUTATION: &str = r"
mutation UnsubscribeFeed($input: UnsubscribeFeedInput!) {
  unsubscribeFeed(input: $input) {
    status { code }
  }
}
";

const REFRESH_FEED_MUTATION: &str = r"
mutation RefreshFeed($input: RefreshFeedInput!) {
  refreshFeed(input: $input) {
    status { code }
    requestId
    disposition
  }
}
";

const TIMELINE_CHANGED_SUBSCRIPTION: &str = r"
subscription TimelineChanged {
  timelineChanged {
    changedAt
    affectedFeeds
  }
}
";
