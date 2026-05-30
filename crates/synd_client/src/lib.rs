#![warn(rustdoc::broken_intra_doc_links)]

use std::{fmt::Debug, time::Duration};

use anyhow::anyhow;
use futures_util::{SinkExt, Stream, StreamExt};
use graphql_client::Response;
use reqwest::header::{self, HeaderValue};
use serde::{Serialize, de::DeserializeOwned};
use synd_protocol::session::{
    CloseSessionErrorResponse, CloseSessionRequest, CloseSessionResponse, OpenSessionErrorResponse,
    OpenSessionRequest, OpenSessionResponse,
};
use synd_support::o11y::{health_check::Health, opentelemetry::extension::*};
use thiserror::Error;
use tokio::{net::TcpStream, sync::mpsc};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream, connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};
use tracing::Span;
use url::Url;

use crate::payload::{
    InitialFeedViewPayload, RefreshFeedPayload, RefreshStatus, SubscribeFeedInput,
    SubscribeFeedPayload, SubscriptionPayload, TimelineChangeEvent,
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
    #[error("session open rejected: {0:?}")]
    OpenSession(OpenSessionErrorResponse),
    #[error("session close rejected: {0:?}")]
    CloseSession(CloseSessionErrorResponse),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, Clone)]
pub struct ClientOptions {
    timeout: Duration,
    user_agent: String,
}

impl ClientOptions {
    pub fn new(timeout: Duration, user_agent: impl Into<String>) -> Self {
        Self {
            timeout,
            user_agent: user_agent.into(),
        }
    }
}

pub enum ApiCredential {
    Github { access_token: String },
    Google { id_token: String },
    LocalBearer { token: String },
}

impl ApiCredential {
    fn into_header_value(self) -> anyhow::Result<HeaderValue> {
        let value = match self {
            Self::Github { access_token } => format!("github {access_token}"),
            Self::Google { id_token } => format!("google {id_token}"),
            Self::LocalBearer { token } => format!("Bearer {token}"),
        };
        let mut value = HeaderValue::try_from(value)?;
        value.set_sensitive(true);
        Ok(value)
    }
}

/// Client for the syndicationd API.
#[derive(Clone)]
pub struct Client {
    #[expect(clippy::struct_field_names)]
    client: reqwest::Client,
    authentication: ClientAuthentication,
    endpoint: Url,
    transport: ClientTransport,
}

#[derive(Clone)]
enum ClientAuthentication {
    Required,
    Header(HeaderValue),
    TransportTrusted,
}

impl ClientAuthentication {
    fn apply_authorization_header(
        &self,
        headers: &mut header::HeaderMap,
    ) -> Result<(), SyndApiError> {
        match self {
            Self::Required => Err(SyndApiError::Internal(anyhow!("Credential not configured"))),
            Self::Header(value) => {
                headers.insert(header::AUTHORIZATION, value.clone());
                Ok(())
            }
            Self::TransportTrusted => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClientTransport {
    Tcp,
    #[cfg(unix)]
    Unix,
}

#[derive(Debug, serde::Deserialize)]
struct NullableEntriesResponseData {
    output: Option<payload::EntriesOutput>,
}

fn fetch_entries_payload_from_response(
    response: Response<NullableEntriesResponseData>,
) -> Result<payload::FetchEntriesPayload, SyndApiError> {
    let errors = response.errors.unwrap_or_default();
    match response.data.and_then(|data| data.output) {
        Some(output) => {
            if !errors.is_empty() {
                tracing::warn!(
                    errors = ?errors,
                    "entries query returned partial GraphQL errors"
                );
            }
            Ok(output.into())
        }
        None if !errors.is_empty() => Err(SyndApiError::Graphql { errors }),
        None => Err(SyndApiError::Internal(anyhow!(
            "Unexpected error. response does not contain data and errors"
        ))),
    }
}

impl Client {
    const GRAPHQL: &'static str = "/graphql";
    const HEALTH_CHECK: &'static str = "/health";
    const SESSION_OPEN: &'static str = "/session/open";
    const SESSION_CLOSE: &'static str = "/session/close";
    const DAEMON_SHUTDOWN: &'static str = "/daemon/shutdown";

    pub fn new(endpoint: Url, options: ClientOptions) -> anyhow::Result<Self> {
        let client = Self::builder(options).build()?;

        Ok(Self {
            client,
            endpoint,
            authentication: ClientAuthentication::Required,
            transport: ClientTransport::Tcp,
        })
    }

    #[cfg(unix)]
    pub fn new_unix(
        socket_path: impl AsRef<std::path::Path>,
        options: ClientOptions,
    ) -> anyhow::Result<Self> {
        let client = Self::builder(options)
            .unix_socket(socket_path.as_ref())
            .build()?;

        Ok(Self {
            client,
            endpoint: Url::parse("http://localhost")?,
            authentication: ClientAuthentication::TransportTrusted,
            transport: ClientTransport::Unix,
        })
    }

    fn builder(options: ClientOptions) -> reqwest::ClientBuilder {
        reqwest::ClientBuilder::new()
            .user_agent(options.user_agent)
            .timeout(options.timeout)
            .connect_timeout(Duration::from_secs(10))
            // this client specifically targets the syndicationd api, so accepts self signed certificates
            .danger_accept_invalid_certs(true)
    }

    pub fn set_credential(&mut self, credential: ApiCredential) -> anyhow::Result<()> {
        self.authentication = ClientAuthentication::Header(credential.into_header_value()?);
        Ok(())
    }

    pub fn set_local_token(&mut self, token: &str) -> anyhow::Result<()> {
        self.set_credential(ApiCredential::LocalBearer {
            token: token.to_owned(),
        })
    }

    pub fn supports_timeline_change_subscription(&self) -> bool {
        self.transport == ClientTransport::Tcp && self.endpoint.scheme() == "http"
    }

    #[tracing::instrument(skip(self))]
    pub async fn fetch_initial_feed_view(
        &self,
        subscriptions_first: i64,
        timeline_first: i64,
    ) -> Result<InitialFeedViewPayload, SyndApiError> {
        #[derive(Serialize, Debug)]
        #[serde(rename_all = "camelCase")]
        struct Variables {
            subscriptions_first: i64,
            timeline_first: i64,
        }
        #[derive(Debug, serde::Deserialize)]
        struct ResponseData {
            output: InitialFeedViewPayload,
        }

        let response: Response<ResponseData> = self
            .execute_graphql(&graphql(
                INITIAL_FEED_VIEW_QUERY,
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
                        "initial feed view query returned partial GraphQL errors"
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

        let response: Response<NullableEntriesResponseData> = self
            .execute_graphql(&graphql(ENTRIES_QUERY, Variables { after, first }))
            .await?;

        tracing::debug!("Got response");

        fetch_entries_payload_from_response(response)
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
            .json(body)
            .build()
            .map_err(SyndApiError::BuildRequest)?;
        self.authentication
            .apply_authorization_header(request.headers_mut())?;

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

    pub async fn shutdown_daemon(&self) -> anyhow::Result<()> {
        self.client
            .post(self.endpoint.join(Self::DAEMON_SHUTDOWN).unwrap())
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    pub async fn open_session(
        &self,
        request: OpenSessionRequest,
    ) -> Result<OpenSessionResponse, SyndApiError> {
        let response = self.execute_json_post(Self::SESSION_OPEN, &request).await?;

        if response.status().is_success() {
            return response.json().await.map_err(SyndApiError::BuildRequest);
        }

        if response.status() == reqwest::StatusCode::CONFLICT {
            return Err(SyndApiError::OpenSession(
                response.json().await.map_err(SyndApiError::BuildRequest)?,
            ));
        }

        response
            .error_for_status()
            .map_err(SyndApiError::BuildRequest)?;

        Err(SyndApiError::Internal(anyhow!(
            "Unexpected session open response"
        )))
    }

    pub async fn close_session(
        &self,
        request: CloseSessionRequest,
    ) -> Result<CloseSessionResponse, SyndApiError> {
        let response = self
            .execute_json_post(Self::SESSION_CLOSE, &request)
            .await?;

        if response.status().is_success() {
            return response.json().await.map_err(SyndApiError::BuildRequest);
        }

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(SyndApiError::CloseSession(
                response.json().await.map_err(SyndApiError::BuildRequest)?,
            ));
        }

        response
            .error_for_status()
            .map_err(SyndApiError::BuildRequest)?;

        Err(SyndApiError::Internal(anyhow!(
            "Unexpected session close response"
        )))
    }

    async fn execute_json_post<T>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, SyndApiError>
    where
        T: Serialize + Debug,
    {
        let mut request = self
            .client
            .post(self.endpoint.join(path).unwrap())
            .json(body)
            .build()
            .map_err(SyndApiError::BuildRequest)?;
        self.authentication
            .apply_authorization_header(request.headers_mut())?;

        self.client
            .execute(request)
            .await
            .map_err(SyndApiError::BuildRequest)
    }

    #[tracing::instrument(skip(self))]
    pub async fn next_timeline_change(&self) -> Result<TimelineChangeEvent, SyndApiError> {
        let mut socket = self.connect_timeline_change_socket().await?;
        wait_for_timeline_change(&mut socket).await
    }

    #[tracing::instrument(skip(self, events))]
    pub async fn run_timeline_changes(
        &self,
        events: mpsc::UnboundedSender<TimelineChangeEvent>,
    ) -> Result<(), SyndApiError> {
        let mut socket = self.connect_timeline_change_socket().await?;

        loop {
            let event = wait_for_timeline_change(&mut socket).await?;
            if events.send(event).is_err() {
                return Ok(());
            }
        }
    }

    async fn connect_timeline_change_socket(
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
        self.authentication
            .apply_authorization_header(request.headers_mut())?;

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

async fn wait_for_timeline_change<S>(socket: &mut S) -> Result<TimelineChangeEvent, SyndApiError>
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
                return Ok(event);
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

#[cfg(test)]
#[expect(clippy::items_after_test_module)]
mod tests {
    use super::{
        ApiCredential, Client, ClientOptions, NullableEntriesResponseData, SyndApiError,
        fetch_entries_payload_from_response, header,
    };

    #[test]
    fn tcp_client_supports_timeline_change_subscription_for_plain_http() {
        let client =
            Client::new(url::Url::parse("http://127.0.0.1:8080").unwrap(), options()).unwrap();

        assert!(client.supports_timeline_change_subscription());
    }

    #[cfg(unix)]
    #[test]
    fn unix_client_does_not_support_timeline_change_subscription_yet() {
        let client = Client::new_unix("/tmp/synd-client-test.sock", options()).unwrap();

        assert!(!client.supports_timeline_change_subscription());
    }

    #[test]
    fn tcp_client_requires_credential_before_authorized_request() {
        let client =
            Client::new(url::Url::parse("http://127.0.0.1:8080").unwrap(), options()).unwrap();
        let mut headers = header::HeaderMap::new();

        let err = client
            .authentication
            .apply_authorization_header(&mut headers)
            .unwrap_err();

        assert!(err.to_string().contains("Credential not configured"));
        assert!(!headers.contains_key(header::AUTHORIZATION));
    }

    #[test]
    fn configured_credential_is_written_to_authorization_header() {
        let mut client =
            Client::new(url::Url::parse("http://127.0.0.1:8080").unwrap(), options()).unwrap();
        client
            .set_credential(ApiCredential::LocalBearer {
                token: "secret".to_owned(),
            })
            .unwrap();
        let mut headers = header::HeaderMap::new();

        client
            .authentication
            .apply_authorization_header(&mut headers)
            .unwrap();

        assert_eq!(headers.get(header::AUTHORIZATION).unwrap(), "Bearer secret");
    }

    #[cfg(unix)]
    #[test]
    fn unix_client_uses_transport_trust_without_authorization_header() {
        let client = Client::new_unix("/tmp/synd-client-test.sock", options()).unwrap();
        let mut headers = header::HeaderMap::new();

        client
            .authentication
            .apply_authorization_header(&mut headers)
            .unwrap();

        assert!(!headers.contains_key(header::AUTHORIZATION));
    }

    #[test]
    fn entries_response_with_graphql_errors_returns_graphql_error() {
        let response = graphql_client::Response::<NullableEntriesResponseData> {
            data: Some(NullableEntriesResponseData { output: None }),
            errors: Some(vec![graphql_client::Error {
                message: "timeline entries are not implemented".to_owned(),
                locations: None,
                path: None,
                extensions: None,
            }]),
            extensions: None,
        };

        let err = fetch_entries_payload_from_response(response).unwrap_err();

        assert!(matches!(err, SyndApiError::Graphql { .. }));
    }

    fn options() -> ClientOptions {
        ClientOptions::new(std::time::Duration::from_secs(1), "synd-client-test")
    }
}

const INITIAL_FEED_VIEW_QUERY: &str = r"
query InitialFeedView($subscriptionsFirst: Int!, $timelineFirst: Int!) {
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
