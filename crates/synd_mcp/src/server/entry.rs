use rmcp::{Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use synd_client::payload::EntryDetails;
use synd_feed::entry::EntryId;

use super::McpServer;
use crate::error::ToolError;

#[derive(Deserialize, JsonSchema)]
struct GetEntryParams {
    /// Stable ID assigned by synd to the entry.
    #[schemars(with = "String")]
    entry_id: EntryId,
}

#[tool_router(router = entry_tools, vis = "pub(super)")]
impl McpServer {
    #[tool(
        description = "Get a stored entry by its entry_id, including its original summary, inline content, links, and metadata.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn get_entry(
        &self,
        Parameters(params): Parameters<GetEntryParams>,
    ) -> Result<Json<EntryDetails>, ToolError> {
        let entry = self
            .session
            .client()
            .await?
            .get_entry(&params.entry_id)
            .await?
            .ok_or(ToolError::EntryNotFound(params.entry_id))?;
        Ok(Json(entry))
    }
}
