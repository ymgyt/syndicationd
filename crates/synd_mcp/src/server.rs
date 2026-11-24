use std::sync::Arc;

use anyhow::Context as _;
use rmcp::{
    ServerHandler, ServiceExt as _,
    handler::server::router::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerConfig},
    service::{QuitReason, ServerInitializeError},
    tool_handler,
    transport::stdio,
};

use crate::session::McpSession;

mod page;
mod subscription;
mod timeline;

/// Routes MCP tool calls through the active synd session.
pub(super) struct McpServer {
    session: Arc<McpSession>,
    tool_router: ToolRouter<Self>,
}

impl McpServer {
    pub(super) fn new(session: Arc<McpSession>) -> Self {
        Self {
            session,
            tool_router: Self::timeline_tools() + Self::subscription_tools(),
        }
    }

    pub(super) async fn serve_stdio(self) -> anyhow::Result<()> {
        let service = match self.serve(stdio()).await {
            Ok(service) => service,
            Err(ServerInitializeError::ConnectionClosed(_)) => return Ok(()),
            Err(error) => return Err(error).context("Failed to initialize MCP server"),
        };
        match service.waiting().await {
            Ok(QuitReason::JoinError(error)) | Err(error) => {
                Err(error).context("MCP server failed")
            }
            Ok(_) => Ok(()),
        }
    }
}

#[tool_handler(router = self.tool_router)]
#[expect(
    clippy::unused_async_trait_impl,
    reason = "rmcp generates async list_tools"
)]
impl ServerHandler for McpServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("synd", env!("CARGO_PKG_VERSION")))
    }
}
