use rmcp::{Json, handler::server::wrapper::Parameters, tool, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use synd_client::{
    FeedUrl, SubscriptionLabels,
    payload::{
        ResponseCode, ResponseStatus, SubscribeDisposition, SubscribeFeedInput,
        SubscribeFeedPayload, SubscribedFeed, SubscriptionPayload, UnsubscribeDisposition,
        UnsubscribeFeedPayload,
    },
};

use super::{
    McpServer,
    page::{Page, PageParams},
};
use crate::error::ToolError;

#[cfg(test)]
mod tests;

#[tool_router(router = subscription_tools, vis = "pub(super)")]
impl McpServer {
    #[tool(
        description = "List subscribed feeds, including feeds that have yet to be fetched.",
        annotations(read_only_hint = true, open_world_hint = false)
    )]
    async fn list_subscriptions(
        &self,
        Parameters(page): Parameters<PageParams>,
    ) -> Result<Json<Page<Subscription>>, ToolError> {
        let first = page.first()?;
        let subscriptions = self
            .session
            .client()
            .await?
            .fetch_subscription(page.into_after(), Some(first))
            .await?;
        Ok(Json(subscriptions.into()))
    }

    #[tool(
        description = "Subscribe to a feed by URL using default settings. An existing subscription is updated to those settings."
    )]
    async fn subscribe_feed(
        &self,
        Parameters(feed): Parameters<FeedParams>,
    ) -> Result<Json<SubscriptionChange>, ToolError> {
        let input = SubscribeFeedInput::try_from(feed)?;
        let response = self.session.client().await?.subscribe_feed(input).await?;
        Ok(Json(response.try_into()?))
    }

    #[tool(
        description = "Unsubscribe from a feed by URL.",
        annotations(destructive_hint = true, open_world_hint = false)
    )]
    async fn unsubscribe_feed(
        &self,
        Parameters(feed): Parameters<FeedParams>,
    ) -> Result<Json<SubscriptionChange>, ToolError> {
        let url = FeedUrl::try_from(feed)?;
        let response = self.session.client().await?.unsubscribe_feed(url).await?;
        Ok(Json(response.try_into()?))
    }
}

#[derive(Serialize, JsonSchema)]
struct Subscription {
    url: String,
    title: Option<String>,
}

impl From<SubscribedFeed> for Subscription {
    fn from(feed: SubscribedFeed) -> Self {
        Self {
            url: feed.url.into_inner().into(),
            title: feed.feed.and_then(|details| details.title),
        }
    }
}

impl From<SubscriptionPayload> for Page<Subscription> {
    fn from(subscription: SubscriptionPayload) -> Self {
        let feeds = subscription.feeds;
        Self::from_nodes(feeds.nodes, feeds.page_info)
    }
}

#[derive(Deserialize, JsonSchema)]
struct FeedParams {
    /// RSS or Atom feed URL.
    url: String,
}

impl TryFrom<FeedParams> for FeedUrl {
    type Error = ToolError;

    fn try_from(params: FeedParams) -> Result<Self, Self::Error> {
        Ok(Self::parse(&params.url)?)
    }
}

impl TryFrom<FeedParams> for SubscribeFeedInput {
    type Error = ToolError;

    fn try_from(params: FeedParams) -> Result<Self, Self::Error> {
        Ok(Self {
            url: params.try_into()?,
            labels: SubscriptionLabels::default(),
            crawl_policy: None,
        })
    }
}

// Reports the outcome of a successful subscribe or unsubscribe, including unfamiliar dispositions.
#[derive(Serialize, JsonSchema)]
struct SubscriptionChange {
    url: String,
    disposition: String,
}

impl SubscriptionChange {
    fn new(status: ResponseStatus, url: FeedUrl, disposition: String) -> Result<Self, ToolError> {
        if status.code != ResponseCode::Ok {
            return Err(ToolError::SubscriptionFailed(status.code));
        }
        Ok(Self {
            url: url.into_inner().into(),
            disposition,
        })
    }
}

impl TryFrom<SubscribeFeedPayload> for SubscriptionChange {
    type Error = ToolError;

    fn try_from(response: SubscribeFeedPayload) -> Result<Self, Self::Error> {
        let disposition = match response.disposition {
            SubscribeDisposition::Subscribed => "subscribed".into(),
            SubscribeDisposition::Changed => "changed".into(),
            SubscribeDisposition::Other(value) => value,
        };
        Self::new(response.status, response.url, disposition)
    }
}

impl TryFrom<UnsubscribeFeedPayload> for SubscriptionChange {
    type Error = ToolError;

    fn try_from(response: UnsubscribeFeedPayload) -> Result<Self, Self::Error> {
        let disposition = match response.disposition {
            UnsubscribeDisposition::Unsubscribed => "unsubscribed".into(),
            UnsubscribeDisposition::Other(value) => value,
        };
        Self::new(response.status, response.url, disposition)
    }
}
