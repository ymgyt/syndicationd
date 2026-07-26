use std::time::Duration;
#[cfg(unix)]
use std::{path::Path, path::PathBuf};

use url::Url;

use crate::SyndApiError;

mod authentication;
mod daemon;
mod feed_events;
mod graphql;
mod session;
mod tls;

pub use authentication::ApiCredential;
use authentication::ClientAuthentication;
pub use feed_events::FeedEventWatch;
use tls::ClientTls;

#[derive(Debug, Clone)]
pub struct ClientOptions {
    timeout: Duration,
    user_agent: String,
    tls: ClientTls,
}

impl ClientOptions {
    pub fn new(timeout: Duration, user_agent: impl Into<String>) -> Self {
        Self {
            timeout,
            user_agent: user_agent.into(),
            tls: ClientTls::default(),
        }
    }

    pub fn with_root_certificate_pem(mut self, pem: &[u8]) -> Result<Self, SyndApiError> {
        self.tls.add_root_certificate_pem(pem)?;
        Ok(self)
    }

    #[must_use]
    pub fn danger_accept_invalid_certs(mut self, accept: bool) -> Self {
        self.tls.accept_invalid_certificates(accept);
        self
    }
}

/// Client for the syndicationd API.
#[derive(Clone)]
pub struct Client {
    #[expect(clippy::struct_field_names)]
    client: reqwest::Client,
    authentication: ClientAuthentication,
    endpoint: Url,
    feed_event_transport: FeedEventTransport,
}

#[derive(Clone)]
enum FeedEventTransport {
    Tcp {
        connector: Option<tokio_tungstenite::Connector>,
    },
    #[cfg(unix)]
    Unix { socket_path: PathBuf },
}

impl Client {
    pub fn new(endpoint: Url, options: ClientOptions) -> Result<Self, SyndApiError> {
        endpoint.join("/")?;
        let connector = options.tls.websocket_connector()?;
        let client = Self::builder(options)?
            .build()
            .map_err(SyndApiError::BuildRequest)?;

        Ok(Self {
            client,
            endpoint,
            authentication: ClientAuthentication::Required,
            feed_event_transport: FeedEventTransport::Tcp { connector },
        })
    }

    #[cfg(unix)]
    pub fn new_unix(
        socket_path: impl AsRef<Path>,
        options: ClientOptions,
    ) -> Result<Self, SyndApiError> {
        let socket_path = socket_path.as_ref().to_path_buf();
        let client = Self::builder(options)?
            .unix_socket(socket_path.clone())
            .build()
            .map_err(SyndApiError::BuildRequest)?;

        Ok(Self {
            client,
            endpoint: Url::parse("http://localhost")?,
            authentication: ClientAuthentication::TransportTrusted,
            feed_event_transport: FeedEventTransport::Unix { socket_path },
        })
    }

    fn builder(options: ClientOptions) -> Result<reqwest::ClientBuilder, SyndApiError> {
        let ClientOptions {
            timeout,
            user_agent,
            tls,
        } = options;
        let builder = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .timeout(timeout)
            .connect_timeout(Duration::from_secs(10));
        tls.configure_http(builder)
    }

    pub fn set_credential(&mut self, credential: ApiCredential) -> Result<(), SyndApiError> {
        self.authentication.configure(credential)
    }

    pub fn set_local_token(&mut self, token: &str) -> Result<(), SyndApiError> {
        self.set_credential(ApiCredential::LocalBearer {
            token: token.to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use core::assert_matches;
    use std::time::Duration;

    use reqwest::header;

    use super::{ApiCredential, Client, ClientOptions};
    use crate::SyndApiError;

    #[test]
    fn tcp_client_requires_credential_before_authorized_request() {
        let client =
            Client::new(url::Url::parse("http://127.0.0.1:8080").unwrap(), options()).unwrap();
        let mut headers = header::HeaderMap::new();

        let error = client
            .authentication
            .apply_authorization_header(&mut headers)
            .unwrap_err();

        assert_matches!(error, SyndApiError::MissingCredential);
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

    fn options() -> ClientOptions {
        ClientOptions::new(Duration::from_secs(1), "synd-client-test")
    }
}
