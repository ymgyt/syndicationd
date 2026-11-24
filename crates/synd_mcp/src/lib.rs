use std::sync::Arc;

use anyhow::Context as _;
use synd_runtime::Runtime;

use self::{server::McpServer, session::McpSession};

mod error;
mod server;
mod session;

/// Serve tools over stdio until the MCP client closes the connection.
pub async fn serve_stdio(runtime: Runtime) -> anyhow::Result<()> {
    let session = Arc::new(
        McpSession::acquire(runtime)
            .await
            .context("Failed to acquire runtime session")?,
    );
    let server = McpServer::new(Arc::clone(&session));

    let result = server.serve_stdio().await;
    session.close().await;
    result
}
