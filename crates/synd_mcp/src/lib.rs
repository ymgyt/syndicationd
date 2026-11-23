use std::sync::Arc;

use anyhow::Context as _;
use rmcp::{
    ServiceExt as _,
    service::{QuitReason, ServerInitializeError},
    transport::stdio,
};
use synd_runtime::Runtime;

use self::{server::McpServer, session::McpSession};

mod server;
mod session;

/// Serve tools over stdio until the MCP client closes the connection.
pub async fn serve_stdio(runtime: Runtime) -> anyhow::Result<()> {
    let session = Arc::new(McpSession::acquire(runtime).await?);
    let server = McpServer::new(Arc::clone(&session));

    let result = match server.serve(stdio()).await {
        Ok(service) => match service.waiting().await {
            Ok(QuitReason::JoinError(error)) | Err(error) => {
                Err(error).context("MCP server failed")
            }
            Ok(_) => Ok(()),
        },
        Err(ServerInitializeError::ConnectionClosed(_)) => Ok(()),
        Err(error) => Err(error).context("Failed to initialize MCP server"),
    };

    session.close().await;
    result
}
