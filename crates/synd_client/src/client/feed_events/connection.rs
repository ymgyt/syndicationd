use reqwest::header::{self, HeaderValue};
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;
#[cfg(unix)]
use tokio_tungstenite::client_async;
use tokio_tungstenite::{
    MaybeTlsStream, connect_async_tls_with_config,
    tungstenite::{client::IntoClientRequest as _, handshake::client::Request},
};
use url::Url;

use super::protocol::FeedEventConnection;
use crate::client::{Client, FeedEventTransport};
use crate::{SyndApiError, payload::FeedEvent};

const FEED_EVENTS_PATH: &str = "/graphql/ws";

/// Active feed-event watch over the client's configured transport.
pub struct FeedEventWatch {
    connection: FeedEventWatchConnection,
}

enum FeedEventWatchConnection {
    Tcp(Box<FeedEventConnection<MaybeTlsStream<TcpStream>>>),
    #[cfg(unix)]
    Unix(Box<FeedEventConnection<UnixStream>>),
}

impl FeedEventWatch {
    pub async fn next_event(&mut self) -> Result<FeedEvent, SyndApiError> {
        match &mut self.connection {
            FeedEventWatchConnection::Tcp(connection) => connection.next_event().await,
            #[cfg(unix)]
            FeedEventWatchConnection::Unix(connection) => connection.next_event().await,
        }
    }
}

impl Client {
    pub async fn watch_feed_events(&self) -> Result<FeedEventWatch, SyndApiError> {
        let request = self.feed_event_request()?;
        match &self.feed_event_transport {
            FeedEventTransport::Tcp { connector } => {
                let (socket, _) =
                    connect_async_tls_with_config(request, None, false, connector.clone())
                        .await
                        .map_err(SyndApiError::WebSocket)?;
                Ok(FeedEventWatch {
                    connection: FeedEventWatchConnection::Tcp(Box::new(
                        FeedEventConnection::establish(socket).await?,
                    )),
                })
            }
            #[cfg(unix)]
            FeedEventTransport::Unix { socket_path } => {
                let stream = UnixStream::connect(socket_path).await.map_err(|error| {
                    SyndApiError::WebSocket(tokio_tungstenite::tungstenite::Error::Io(error))
                })?;
                let (socket, _) = client_async(request, stream)
                    .await
                    .map_err(SyndApiError::WebSocket)?;
                Ok(FeedEventWatch {
                    connection: FeedEventWatchConnection::Unix(Box::new(
                        FeedEventConnection::establish(socket).await?,
                    )),
                })
            }
        }
    }

    fn feed_event_request(&self) -> Result<Request, SyndApiError> {
        let mut request = self
            .graphql_ws_endpoint()?
            .as_str()
            .into_client_request()
            .map_err(SyndApiError::WebSocket)?;
        request.headers_mut().insert(
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue::from_static("graphql-transport-ws"),
        );
        self.authentication
            .apply_authorization_header(request.headers_mut())?;
        Ok(request)
    }

    fn graphql_ws_endpoint(&self) -> Result<Url, SyndApiError> {
        let mut url = self.endpoint.join(FEED_EVENTS_PATH)?;
        let scheme = match url.scheme() {
            "http" => "ws",
            "https" => "wss",
            scheme => {
                return Err(SyndApiError::UnsupportedWebSocketScheme {
                    scheme: scheme.to_owned(),
                });
            }
        };
        url.set_scheme(scheme)
            .map_err(|()| SyndApiError::SetWebSocketScheme)?;
        Ok(url)
    }
}
