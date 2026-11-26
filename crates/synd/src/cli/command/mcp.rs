use clap::Args;

use crate::{config::ConfigResolver, runtime::FeedRuntime};

/// Serve MCP tools over standard input and output
#[derive(Args, Debug)]
pub struct McpCommand;

impl McpCommand {
    pub async fn run(self, config: &ConfigResolver) -> anyhow::Result<()> {
        let runtime = FeedRuntime::new(config)?;
        synd_mcp::serve_stdio(runtime.into_runtime()).await
    }
}
