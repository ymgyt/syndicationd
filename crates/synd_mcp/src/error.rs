use rmcp::model::{ContentBlock, IntoContents};
use synd_client::{SyndApiError, payload::ResponseCode};
use synd_feed::entry::EntryId;
use synd_runtime::Error as RuntimeError;
use synd_support::html::fetch::ArticleFetchError;
use thiserror::Error;
use url::ParseError;

/// Failures reported to the MCP client as tool errors.
#[derive(Debug, Error)]
pub(crate) enum ToolError {
    #[error("Entry not found: {0}")]
    EntryNotFound(EntryId),
    #[error(transparent)]
    FetchEntryBody(#[from] ArticleFetchError),
    #[error("first must be between 1 and 100")]
    InvalidPageSize,
    #[error("Invalid feed URL: {0}")]
    InvalidFeedUrl(#[from] ParseError),
    #[error("Subscription change failed: {0:?}")]
    SubscriptionFailed(ResponseCode),
    #[error(transparent)]
    Api(#[from] SyndApiError),
    #[error("MCP session is closed")]
    SessionClosed,
    #[error("Failed to reacquire runtime session")]
    ReacquireSession(#[source] RuntimeError),
}

impl IntoContents for ToolError {
    fn into_contents(self) -> Vec<ContentBlock> {
        let message = match self {
            Self::ReacquireSession(_) | Self::FetchEntryBody(_) => {
                format!("{:#}", anyhow::Error::new(self))
            }
            error => error.to_string(),
        };
        message.into_contents()
    }
}
