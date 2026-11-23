use std::sync::Arc;

use rmcp::{
    Json, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synd_client::{
    Client, FeedUrl,
    payload::{
        PageInfo, ResponseCode, ResponseStatus, SubscribeDisposition, SubscribeFeedInput,
        TimelineId, UnsubscribeDisposition,
    },
};

use super::session::McpSession;

pub(super) struct McpServer {
    session: Arc<McpSession>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl McpServer {
    pub(super) fn new(session: Arc<McpSession>) -> Self {
        Self {
            session,
            tool_router: Self::tool_router(),
        }
    }

    #[tool(
        description = "List timelines with their IDs and names. Use an ID with list_timeline_entries.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_timelines(&self) -> Result<Json<TimelineList>, String> {
        let timelines = self
            .client()
            .await?
            .list_timelines()
            .await
            .map_err(|error| error.to_string())?;

        Ok(Json(TimelineList {
            items: timelines
                .into_iter()
                .map(|timeline| Timeline {
                    id: timeline.id,
                    name: timeline.name.to_string(),
                })
                .collect(),
        }))
    }

    #[tool(
        description = "List entries in the specified timeline from newest to oldest with their titles, summaries, and URLs.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_timeline_entries(
        &self,
        Parameters(params): Parameters<TimelineEntriesParams>,
    ) -> Result<Json<Page<TimelineEntry>>, String> {
        let first = params.page.first()?;
        let connection = self
            .client()
            .await?
            .fetch_timeline_entries(params.timeline_id, params.page.after, first)
            .await
            .map_err(|error| error.to_string())?;

        Ok(Json(Page::new(
            connection.nodes.into_iter().map(|node| TimelineEntry {
                title: node.entry.title,
                summary: node.entry.summary,
                url: node.entry.website_url,
            }),
            connection.page_info,
        )))
    }

    #[tool(
        description = "List subscribed feeds, including feeds that have yet to be fetched.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_subscriptions(
        &self,
        Parameters(page): Parameters<PageParams>,
    ) -> Result<Json<Page<Subscription>>, String> {
        let first = page.first()?;
        let connection = self
            .client()
            .await?
            .fetch_subscription(page.after, Some(first))
            .await
            .map_err(|error| error.to_string())?
            .feeds;

        Ok(Json(Page::new(
            connection.nodes.into_iter().map(|node| Subscription {
                url: node.url.to_string(),
                title: node.feed.and_then(|feed| feed.title),
            }),
            connection.page_info,
        )))
    }

    #[tool(
        description = "Subscribe to a feed by URL using default settings. An existing subscription is updated to those settings."
    )]
    async fn subscribe_feed(
        &self,
        Parameters(feed): Parameters<FeedParams>,
    ) -> Result<Json<SubscriptionChange>, String> {
        let url = feed.parse_url()?;
        let response = self
            .client()
            .await?
            .subscribe_feed(SubscribeFeedInput {
                url,
                requirement: None,
                category: None,
                crawl_policy: None,
            })
            .await
            .map_err(|error| error.to_string())?;
        let disposition = match response.disposition {
            SubscribeDisposition::Subscribed => "subscribed".into(),
            SubscribeDisposition::Changed => "changed".into(),
            SubscribeDisposition::Other(disposition) => disposition,
        };
        SubscriptionChange::result(&response.status, &response.url, disposition)
    }

    #[tool(
        description = "Unsubscribe from a feed by URL.",
        annotations(destructive_hint = true, open_world_hint = false)
    )]
    async fn unsubscribe_feed(
        &self,
        Parameters(feed): Parameters<FeedParams>,
    ) -> Result<Json<SubscriptionChange>, String> {
        let url = feed.parse_url()?;
        let response = self
            .client()
            .await?
            .unsubscribe_feed(url)
            .await
            .map_err(|error| error.to_string())?;
        let disposition = match response.disposition {
            UnsubscribeDisposition::Unsubscribed => "unsubscribed".into(),
            UnsubscribeDisposition::Other(disposition) => disposition,
        };
        SubscriptionChange::result(&response.status, &response.url, disposition)
    }
}

impl McpServer {
    async fn client(&self) -> Result<Client, String> {
        self.session
            .client()
            .await
            .map_err(|error| format!("{error:#}"))
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

#[derive(Deserialize, JsonSchema)]
struct PageParams {
    /// Cursor returned as `next_cursor` by the previous page of this tool.
    after: Option<String>,
    /// Maximum number of items to return. Defaults to 20.
    #[schemars(range(min = 1, max = 100))]
    first: Option<i64>,
}

impl PageParams {
    fn first(&self) -> Result<i64, String> {
        match self.first.unwrap_or(20) {
            first @ 1..=100 => Ok(first),
            _ => Err("first must be between 1 and 100".into()),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
struct TimelineEntriesParams {
    /// ID returned by `list_timelines`.
    timeline_id: TimelineId,
    #[serde(flatten)]
    page: PageParams,
}

#[derive(Serialize, JsonSchema)]
struct Page<T> {
    items: Vec<T>,
    /// Pass this as `after` to fetch the next page. Null on the last page.
    next_cursor: Option<String>,
}

impl<T> Page<T> {
    fn new(items: impl Iterator<Item = T>, page_info: PageInfo) -> Self {
        let next_cursor = match page_info {
            PageInfo::Complete { .. } => None,
            PageInfo::More { next_cursor } => Some(next_cursor),
        };
        Self {
            items: items.collect(),
            next_cursor,
        }
    }
}

#[derive(Serialize, JsonSchema)]
struct TimelineList {
    items: Vec<Timeline>,
}

#[derive(Serialize, JsonSchema)]
struct Timeline {
    id: TimelineId,
    name: String,
}

#[derive(Serialize, JsonSchema)]
struct TimelineEntry {
    title: Option<String>,
    summary: Option<String>,
    url: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct Subscription {
    url: String,
    title: Option<String>,
}

#[derive(Deserialize, JsonSchema)]
struct FeedParams {
    /// RSS or Atom feed URL.
    url: String,
}

impl FeedParams {
    fn parse_url(&self) -> Result<FeedUrl, String> {
        FeedUrl::parse(&self.url).map_err(|error| format!("Invalid feed URL: {error}"))
    }
}

#[derive(Serialize, JsonSchema)]
struct SubscriptionChange {
    url: String,
    disposition: String,
}

impl SubscriptionChange {
    fn result(
        status: &ResponseStatus,
        url: &FeedUrl,
        disposition: String,
    ) -> Result<Json<Self>, String> {
        if status.code != ResponseCode::Ok {
            return Err(format!("Subscription change failed: {:?}", status.code));
        }

        Ok(Json(Self {
            url: url.to_string(),
            disposition,
        }))
    }
}
