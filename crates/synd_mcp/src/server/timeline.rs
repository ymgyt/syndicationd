use rmcp::{Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synd_client::{
    Rfc3339Time,
    payload::{self, Feed, TimelineEntriesRequest, TimelineEntryConnection, TimelineId},
};
use synd_feed::entry::EntryId;

use super::{
    McpServer,
    page::{Page, PageParams},
};
use crate::error::ToolError;

#[tool_router(router = timeline_tools, vis = "pub(super)")]
impl McpServer {
    #[tool(
        description = "List timelines with their IDs and names. Use an ID with list_timeline_entries.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_timelines(&self) -> Result<Json<TimelineList>, ToolError> {
        let timelines = self.session.client().await?.list_timelines().await?;
        Ok(Json(timelines.into()))
    }

    #[tool(
        description = "List timeline entries newest first by orderTime, with titles, plain-text excerpts, article URLs, feeds, and publication times. When paging through a time range, repeat the same since and until bounds. To read an article, pass its url to fetch_entry_body.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_timeline_entries(
        &self,
        Parameters(params): Parameters<TimelineEntriesParams>,
    ) -> Result<Json<Page<TimelineEntry>>, ToolError> {
        let request = TimelineEntriesRequest::try_from(params)?;
        let entries = self
            .session
            .client()
            .await?
            .fetch_timeline_entries(request)
            .await?;
        Ok(Json(entries.into()))
    }
}

#[derive(Serialize, JsonSchema)]
struct TimelineList {
    items: Vec<Timeline>,
}

impl From<Vec<payload::Timeline>> for TimelineList {
    fn from(timelines: Vec<payload::Timeline>) -> Self {
        Self {
            items: timelines.into_iter().map(Timeline::from).collect(),
        }
    }
}

#[derive(Serialize, JsonSchema)]
struct Timeline {
    id: TimelineId,
    name: String,
}

impl From<payload::Timeline> for Timeline {
    fn from(timeline: payload::Timeline) -> Self {
        Self {
            id: timeline.id,
            name: timeline.name.to_string(),
        }
    }
}

#[derive(Deserialize, JsonSchema)]
struct TimelineEntriesParams {
    /// ID returned by `list_timelines`.
    timeline_id: TimelineId,
    /// Include entries with `orderTime` at or after this RFC 3339 timestamp.
    since: Option<Rfc3339Time>,
    /// Include entries with `orderTime` before this RFC 3339 timestamp.
    until: Option<Rfc3339Time>,
    /// Maximum Unicode display width per excerpt. Defaults to 200.
    #[serde(rename = "maxUnicodeWidth")]
    #[schemars(range(min = 0, max = 1000))]
    max_unicode_width: Option<u16>,
    #[serde(flatten)]
    page: PageParams,
}

impl TryFrom<TimelineEntriesParams> for TimelineEntriesRequest {
    type Error = ToolError;

    fn try_from(params: TimelineEntriesParams) -> Result<Self, Self::Error> {
        Ok(Self::builder()
            .timeline_id(params.timeline_id)
            .first(params.page.first()?)
            .maybe_after(params.page.into_after())
            .maybe_since(params.since)
            .maybe_until(params.until)
            .maybe_max_unicode_width(params.max_unicode_width)
            .build())
    }
}

/// An article in the timeline and the feed it came from.
#[derive(Serialize, JsonSchema)]
struct TimelineEntry {
    /// Stable ID assigned by synd to the entry.
    #[schemars(with = "String")]
    id: EntryId,
    title: Option<String>,
    excerpt: Option<String>,
    /// URL of the article on the publisher's website.
    url: Option<String>,
    feed: Feed,
    /// Publication time supplied by the feed, if available.
    published: Option<Rfc3339Time>,
    /// Sort time fixed when synd first sees the entry. Uses its publication time,
    /// update time, or first observation time, in that order.
    #[serde(rename = "orderTime")]
    order_time: Rfc3339Time,
}

impl From<payload::TimelineEntry> for TimelineEntry {
    fn from(node: payload::TimelineEntry) -> Self {
        let entry = node.entry;
        Self {
            id: entry.id,
            title: entry.title,
            excerpt: entry.excerpt.map(|excerpt| excerpt.text),
            url: entry.website_url,
            feed: entry.feed.into(),
            published: entry.published,
            order_time: node.order_time,
        }
    }
}

impl From<TimelineEntryConnection> for Page<TimelineEntry> {
    fn from(connection: TimelineEntryConnection) -> Self {
        Self::from_nodes(connection.nodes, connection.page_info)
    }
}
