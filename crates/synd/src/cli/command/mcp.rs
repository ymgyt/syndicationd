use std::process::ExitCode;

use clap::Args;

use crate::{config::ConfigResolver, runtime::FeedRuntime};

use super::CommandFailure;

/// Serve MCP tools over standard input and output
#[derive(Args, Debug)]
pub struct McpCommand;

impl McpCommand {
    pub async fn run(self, config: &ConfigResolver) -> ExitCode {
        let result = async {
            let runtime = FeedRuntime::new(config)?;
            synd_mcp::serve_stdio(runtime.into_runtime()).await
        }
        .await;

        match result {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => CommandFailure::report(error),
        }
    }
}
