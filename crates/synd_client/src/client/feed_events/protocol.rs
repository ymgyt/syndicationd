use futures_util::{SinkExt as _, StreamExt as _};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};

use crate::{SyndApiError, payload::FeedEvent};

const FEED_EVENTS_OPERATION_ID: &str = "feedEvents";
const FEED_EVENTS_SUBSCRIPTION: &str = include_str!("feed_events.gql");

/// Converts WebSocket frames to and from GraphQL transport messages.
struct GraphqlWsSocket<S> {
    inner: WebSocketStream<S>,
}

impl<S> GraphqlWsSocket<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    fn new(inner: WebSocketStream<S>) -> Self {
        Self { inner }
    }

    async fn send(&mut self, message: GraphqlWsCommand) -> Result<(), SyndApiError> {
        let message = serde_json::to_string(&message).map_err(SyndApiError::Json)?;
        self.inner
            .send(Message::text(message))
            .await
            .map_err(SyndApiError::WebSocket)
    }

    async fn receive(&mut self) -> Result<GraphqlWsMessage, SyndApiError> {
        loop {
            let message = self
                .inner
                .next()
                .await
                .ok_or_else(|| SyndApiError::FeedEventWatchClosed {
                    code: None,
                    reason: "WebSocket stream ended".to_owned(),
                })?
                .map_err(SyndApiError::WebSocket)?;
            match message {
                Message::Text(text) => {
                    return serde_json::from_str(text.as_ref()).map_err(SyndApiError::Json);
                }
                Message::Binary(bytes) => {
                    return serde_json::from_slice(&bytes).map_err(SyndApiError::Json);
                }
                Message::Close(frame) => {
                    let (code, reason) = frame.map_or((None, String::new()), |frame| {
                        (Some(frame.code.into()), frame.reason.to_string())
                    });
                    return Err(SyndApiError::FeedEventWatchClosed { code, reason });
                }
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
            }
        }
    }
}

/// WebSocket connection with an acknowledged, active feed-event operation.
pub(super) struct FeedEventConnection<S> {
    socket: GraphqlWsSocket<S>,
    operation_id: &'static str,
}

impl<S> FeedEventConnection<S>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub(super) async fn establish(socket: WebSocketStream<S>) -> Result<Self, SyndApiError> {
        let mut socket = GraphqlWsSocket::new(socket);
        socket.send(GraphqlWsCommand::ConnectionInit).await?;

        loop {
            match socket.receive().await? {
                GraphqlWsMessage::ConnectionAck => break,
                GraphqlWsMessage::Ping { payload } => {
                    socket.send(GraphqlWsCommand::Pong { payload }).await?;
                }
                GraphqlWsMessage::Pong => {}
                GraphqlWsMessage::ConnectionError { payload }
                | GraphqlWsMessage::Error { payload, .. } => {
                    return Err(SyndApiError::SubscriptionProtocol {
                        message: format!("connection failed: {payload}"),
                    });
                }
                GraphqlWsMessage::Next { .. }
                | GraphqlWsMessage::Complete { .. }
                | GraphqlWsMessage::Unknown => {
                    return Err(SyndApiError::SubscriptionProtocol {
                        message: "unexpected message before connection acknowledgement".to_owned(),
                    });
                }
            }
        }

        socket
            .send(GraphqlWsCommand::Subscribe {
                id: FEED_EVENTS_OPERATION_ID,
                payload: SubscribePayload {
                    query: FEED_EVENTS_SUBSCRIPTION,
                },
            })
            .await?;

        Ok(Self {
            socket,
            operation_id: FEED_EVENTS_OPERATION_ID,
        })
    }

    pub(super) async fn next_event(&mut self) -> Result<FeedEvent, SyndApiError> {
        loop {
            match self.socket.receive().await? {
                GraphqlWsMessage::Next {
                    id,
                    payload: Some(payload),
                } => {
                    self.require_operation_id(id.as_deref())?;
                    return payload.try_into();
                }
                GraphqlWsMessage::Next { id, payload: None } => {
                    self.require_operation_id(id.as_deref())?;
                    return Err(SyndApiError::SubscriptionProtocol {
                        message: "missing payload".to_owned(),
                    });
                }
                GraphqlWsMessage::ConnectionError { payload } => {
                    return Err(SyndApiError::SubscriptionProtocol {
                        message: format!("subscription error: {payload}"),
                    });
                }
                GraphqlWsMessage::Error { id, payload } => {
                    self.require_operation_id(id.as_deref())?;
                    return Err(SyndApiError::SubscriptionProtocol {
                        message: format!("subscription error: {payload}"),
                    });
                }
                GraphqlWsMessage::Complete { id } => {
                    self.require_operation_id(id.as_deref())?;
                    return Err(SyndApiError::FeedEventWatchClosed {
                        code: None,
                        reason: "server completed the feed-event operation".to_owned(),
                    });
                }
                GraphqlWsMessage::Ping { payload } => {
                    self.socket.send(GraphqlWsCommand::Pong { payload }).await?;
                }
                GraphqlWsMessage::Pong => {}
                GraphqlWsMessage::ConnectionAck | GraphqlWsMessage::Unknown => {
                    return Err(SyndApiError::SubscriptionProtocol {
                        message: "unexpected message on active feed-event operation".to_owned(),
                    });
                }
            }
        }
    }

    fn require_operation_id(&self, actual: Option<&str>) -> Result<(), SyndApiError> {
        if actual == Some(self.operation_id) {
            return Ok(());
        }
        Err(SyndApiError::SubscriptionProtocol {
            message: format!(
                "unexpected operation id: expected {}, received {}",
                self.operation_id,
                actual.unwrap_or("<missing>")
            ),
        })
    }
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GraphqlWsCommand {
    ConnectionInit,
    Subscribe {
        id: &'static str,
        payload: SubscribePayload,
    },
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        payload: Option<serde_json::Value>,
    },
}

#[derive(Serialize)]
struct SubscribePayload {
    query: &'static str,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum GraphqlWsMessage {
    ConnectionAck,
    ConnectionError {
        #[serde(default)]
        payload: serde_json::Value,
    },
    Next {
        id: Option<String>,
        payload: Option<FeedEventPayload>,
    },
    Error {
        id: Option<String>,
        #[serde(default)]
        payload: serde_json::Value,
    },
    Complete {
        id: Option<String>,
    },
    Ping {
        #[serde(default)]
        payload: Option<serde_json::Value>,
    },
    Pong,
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize)]
struct FeedEventPayload {
    #[serde(default)]
    errors: Vec<graphql_client::Error>,
    data: Option<FeedEventData>,
}

impl TryFrom<FeedEventPayload> for FeedEvent {
    type Error = SyndApiError;

    fn try_from(payload: FeedEventPayload) -> Result<Self, Self::Error> {
        if !payload.errors.is_empty() {
            return Err(SyndApiError::Graphql {
                errors: payload.errors,
            });
        }
        payload
            .data
            .and_then(|data| data.feed_event)
            .ok_or_else(|| SyndApiError::SubscriptionProtocol {
                message: "missing feedEvents payload".to_owned(),
            })
    }
}

#[derive(Deserialize)]
struct FeedEventData {
    #[serde(rename = "feedEvents")]
    feed_event: Option<FeedEvent>,
}
