use anyhow::Context as _;
use synd_client::{Client, SyndApiError};
use synd_runtime::{Runtime, Session};
use tokio::sync::Mutex;
use tracing::warn;

pub(super) struct McpSession {
    runtime: Runtime,
    session: Mutex<Option<Session>>,
}

impl McpSession {
    pub(super) async fn acquire(runtime: Runtime) -> anyhow::Result<Self> {
        let session = runtime
            .acquire_session()
            .await
            .context("Failed to acquire runtime session")?;
        Ok(Self {
            runtime,
            session: Mutex::new(Some(session)),
        })
    }

    pub(super) async fn client(&self) -> anyhow::Result<Client> {
        let mut current = self.session.lock().await;
        let session = current.as_mut().context("MCP session is closed")?;

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
                .context("Failed to reacquire runtime session")?;
            close(std::mem::replace(session, replacement)).await;
        }

        Ok(session.client().clone())
    }

    pub(super) async fn close(&self) {
        let session = self.session.lock().await.take();
        if let Some(session) = session {
            close(session).await;
        }
    }
}

async fn close(session: Session) {
    match session.close().await {
        Ok(()) => {}
        Err(synd_runtime::Error::Api(error)) if matches!(*error, SyndApiError::CloseSession(_)) => {
        }
        Err(error) => warn!(?error, "Failed to close runtime session"),
    }
}
