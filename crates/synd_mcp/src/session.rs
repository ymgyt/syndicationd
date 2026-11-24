use std::mem;

use synd_client::{Client, SyndApiError};
use synd_runtime::{Error as RuntimeError, Runtime, Session};
use tokio::sync::Mutex;
use tracing::warn;

use crate::error::ToolError;

/// Maintains the runtime session shared by MCP tool calls.
pub(super) struct McpSession {
    runtime: Runtime,
    session: Mutex<Option<Session>>,
}

impl McpSession {
    pub(super) async fn acquire(runtime: Runtime) -> Result<Self, RuntimeError> {
        let session = runtime.acquire_session().await?;
        Ok(Self {
            runtime,
            session: Mutex::new(Some(session)),
        })
    }

    pub(super) async fn client(&self) -> Result<Client, ToolError> {
        let mut current = self.session.lock().await;
        let session = current.as_mut().ok_or(ToolError::SessionClosed)?;

        // A restarted daemon can serve GraphQL at the same endpoint without
        // knowing the old session ID. Confirm the lease before sending a tool call.
        if let Err(error) = session.renew_lease().await {
            warn!(
                ?error,
                "Reacquiring runtime session after lease renewal failed"
            );
            let replacement = self
                .runtime
                .acquire_session()
                .await
                .map_err(ToolError::ReacquireSession)?;
            let previous = mem::replace(session, replacement);
            Self::close_runtime_session(previous).await;
        }

        Ok(session.client().clone())
    }

    pub(super) async fn close(&self) {
        let session = self.session.lock().await.take();
        if let Some(session) = session {
            Self::close_runtime_session(session).await;
        }
    }

    async fn close_runtime_session(session: Session) {
        match session.close().await {
            Ok(()) => {}
            Err(RuntimeError::Api(error)) if matches!(*error, SyndApiError::CloseSession(_)) => {}
            Err(error) => warn!(?error, "Failed to close runtime session"),
        }
    }
}
