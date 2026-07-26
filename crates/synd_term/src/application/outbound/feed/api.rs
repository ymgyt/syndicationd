use std::sync::Arc;

use futures_util::future::BoxFuture;
use synd_client::{ApiCredential, SyndApiError, payload};
use synd_feed::types::FeedUrl;

pub type FeedApiRef = Arc<dyn FeedApi>;

/// Active feed-event watch returned by a `FeedApi`.
pub trait FeedEventWatch: Send {
    fn next_event(&mut self) -> BoxFuture<'_, Result<payload::FeedEvent, SyndApiError>>;
}

/// Outbound feed capability required by the terminal application workflow.
pub trait FeedApi: Send + Sync + 'static {
    fn set_credential(&self, credential: ApiCredential) -> Result<(), SyndApiError>;

    fn fetch_subscription(
        &self,
        after: Option<String>,
        first: Option<i64>,
    ) -> BoxFuture<'static, Result<payload::SubscriptionPayload, SyndApiError>>;

    fn subscribe_feed(
        &self,
        input: payload::SubscribeFeedInput,
    ) -> BoxFuture<'static, Result<payload::SubscribeFeedPayload, SyndApiError>>;

    fn unsubscribe_feed(&self, url: FeedUrl) -> BoxFuture<'static, Result<(), SyndApiError>>;

    fn fetch_timeline_entries(
        &self,
        after: Option<String>,
        first: i64,
    ) -> BoxFuture<'static, Result<payload::TimelineEntryConnection, SyndApiError>>;

    fn fetch_timeline_changes(
        &self,
        since: i64,
        first: i64,
    ) -> BoxFuture<'static, Result<payload::TimelineChangesPayload, SyndApiError>>;

    fn watch_feed_events(
        &self,
    ) -> BoxFuture<'static, Result<Box<dyn FeedEventWatch>, SyndApiError>>;
}
