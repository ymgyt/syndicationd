use rmcp::{Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synd_support::html::fetch::FetchedArticle;
use url::Url;

use super::McpServer;
use crate::error::ToolError;

#[derive(Deserialize, JsonSchema)]
struct FetchEntryBodyParams {
    /// HTTP or HTTPS URL of the entry page.
    url: Url,
}

#[derive(Serialize, JsonSchema)]
struct EntryBody {
    /// URL of the page after redirects.
    url: Url,
    /// Markdown body extracted by the Reader.
    body: String,
}

impl From<FetchedArticle> for EntryBody {
    fn from(fetched: FetchedArticle) -> Self {
        Self {
            url: fetched.url,
            body: fetched.article.into_markdown(),
        }
    }
}

#[tool_router(router = entry_tools, vis = "pub(super)")]
impl McpServer {
    #[tool(
        description = "Fetch an HTML or XHTML entry page by URL. Return its final URL and the body as Markdown using the same extraction as the Reader.",
        annotations(read_only_hint = true, open_world_hint = true)
    )]
    async fn fetch_entry_body(
        &self,
        Parameters(params): Parameters<FetchEntryBodyParams>,
    ) -> Result<Json<EntryBody>, ToolError> {
        let fetched = self.article_fetcher.fetch(params.url).await?;
        Ok(Json(fetched.into()))
    }
}
