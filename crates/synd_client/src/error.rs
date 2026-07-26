use reqwest::StatusCode;
use synd_protocol::session::{
    CloseSessionErrorResponse, OpenSessionErrorResponse, RenewSessionErrorResponse,
};
use thiserror::Error;
use tokio_tungstenite::tungstenite;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Retryability {
    Retryable,
    Permanent,
}

#[derive(Error, Debug)]
pub enum SyndApiError {
    #[error("unauthorized")]
    Unauthorized { url: Option<Url> },
    #[error(transparent)]
    BuildRequest(reqwest::Error),
    #[error(transparent)]
    SendRequest(reqwest::Error),
    #[error(transparent)]
    DecodeResponse(reqwest::Error),
    #[error("HTTP status client error ({status}) for url ({url})", url = url.as_ref().map(ToString::to_string).unwrap_or_default())]
    HttpStatus {
        status: StatusCode,
        url: Option<Url>,
    },
    #[error("graphql error: {errors:?}")]
    Graphql { errors: Vec<graphql_client::Error> },
    #[error("session open rejected: {0}")]
    OpenSession(OpenSessionErrorResponse),
    #[error("session renew rejected: {0}")]
    RenewSession(RenewSessionErrorResponse),
    #[error("session close rejected: {0}")]
    CloseSession(CloseSessionErrorResponse),
    #[error("credential is not configured")]
    MissingCredential,
    #[error("invalid authorization header")]
    InvalidHeader(#[from] reqwest::header::InvalidHeaderValue),
    #[error("invalid url")]
    InvalidUrl(#[from] url::ParseError),
    #[error("invalid root certificate: {message}")]
    InvalidRootCertificate { message: String },
    #[error("invalid TLS configuration: {message}")]
    TlsConfiguration { message: String },
    #[error("websocket error: {0}")]
    WebSocket(#[from] tungstenite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("unexpected response: {context}")]
    UnexpectedResponse { context: &'static str },
    #[error("unsupported GraphQL subscription endpoint scheme: {scheme}")]
    UnsupportedWebSocketScheme { scheme: String },
    #[error("failed to set websocket scheme")]
    SetWebSocketScheme,
    #[error("GraphQL subscription protocol error: {message}")]
    SubscriptionProtocol { message: String },
    #[error("feed-event watch closed (code {code:?}): {reason}")]
    FeedEventWatchClosed { code: Option<u16>, reason: String },
}

impl SyndApiError {
    pub fn retryability(&self) -> Retryability {
        match self {
            Self::SendRequest(error) if Self::request_error_is_retryable(error) => {
                Retryability::Retryable
            }
            Self::DecodeResponse(error) if Self::response_error_is_retryable(error) => {
                Retryability::Retryable
            }
            Self::FeedEventWatchClosed { code, .. } if Self::close_code_is_retryable(*code) => {
                Retryability::Retryable
            }
            Self::HttpStatus { status, .. }
                if status.is_server_error() || matches!(status.as_u16(), 408 | 425 | 429) =>
            {
                Retryability::Retryable
            }
            Self::WebSocket(error) if Self::websocket_error_is_retryable(error) => {
                Retryability::Retryable
            }
            Self::Unauthorized { .. }
            | Self::BuildRequest(_)
            | Self::SendRequest(_)
            | Self::DecodeResponse(_)
            | Self::HttpStatus { .. }
            | Self::Graphql { .. }
            | Self::OpenSession(_)
            | Self::RenewSession(_)
            | Self::CloseSession(_)
            | Self::MissingCredential
            | Self::InvalidHeader(_)
            | Self::InvalidUrl(_)
            | Self::InvalidRootCertificate { .. }
            | Self::TlsConfiguration { .. }
            | Self::WebSocket(_)
            | Self::Json(_)
            | Self::UnexpectedResponse { .. }
            | Self::UnsupportedWebSocketScheme { .. }
            | Self::SetWebSocketScheme
            | Self::SubscriptionProtocol { .. }
            | Self::FeedEventWatchClosed { .. } => Retryability::Permanent,
        }
    }

    pub(super) fn from_send_error(error: reqwest::Error) -> Self {
        if error.is_builder() {
            Self::BuildRequest(error)
        } else {
            Self::SendRequest(error)
        }
    }

    pub(super) fn from_status_error(error: reqwest::Error) -> Self {
        match error.status() {
            Some(StatusCode::UNAUTHORIZED) => Self::Unauthorized {
                url: error.url().cloned(),
            },
            Some(status) => Self::HttpStatus {
                status,
                url: error.url().cloned(),
            },
            None => Self::from_send_error(error),
        }
    }

    fn websocket_error_is_retryable(error: &tungstenite::Error) -> bool {
        match error {
            tungstenite::Error::ConnectionClosed | tungstenite::Error::Io(_) => true,
            tungstenite::Error::Http(response) => {
                response.status().is_server_error()
                    || matches!(response.status().as_u16(), 408 | 425 | 429)
            }
            tungstenite::Error::AlreadyClosed
            | tungstenite::Error::Tls(_)
            | tungstenite::Error::Capacity(_)
            | tungstenite::Error::Protocol(_)
            | tungstenite::Error::WriteBufferFull(_)
            | tungstenite::Error::Utf8(_)
            | tungstenite::Error::AttackAttempt
            | tungstenite::Error::Url(_)
            | tungstenite::Error::HttpFormat(_) => false,
        }
    }

    fn request_error_is_retryable(error: &reqwest::Error) -> bool {
        !error.is_redirect()
            && (error.is_connect() || error.is_timeout() || error.is_body() || error.is_request())
    }

    fn response_error_is_retryable(error: &reqwest::Error) -> bool {
        error.is_connect() || error.is_timeout() || (error.is_body() && !error.is_decode())
    }

    fn close_code_is_retryable(code: Option<u16>) -> bool {
        match code {
            None | Some(1000 | 1001 | 1006 | 1011 | 1012 | 1013 | 1014 | 4408 | 4500 | 4504) => {
                true
            }
            Some(_) => false,
        }
    }
}
