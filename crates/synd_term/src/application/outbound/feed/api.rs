use std::sync::Arc;

use futures_util::future::BoxFuture;
use synd_client::{ApiCredential, SyndApiError, payload};
use synd_feed::types::FeedUrl;
use tokio::sync::mpsc;

pub type FeedApiRef = Arc<dyn FeedApi>;

/// Outbound feed capability required by the terminal application workflow.
pub trait FeedApi: Send + Sync + 'static {
    fn set_credential(&self, credential: ApiCredential) -> Result<(), SyndApiError>;

    fn supports_timeline_change_subscription(&self) -> bool;

    fn fetch_initial_feed_view(
        &self,
        subscriptions_first: i64,
        timeline_first: i64,
    ) -> BoxFuture<'static, Result<payload::InitialFeedViewPayload, SyndApiError>>;

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

    fn refresh_feed(
        &self,
        url: FeedUrl,
    ) -> BoxFuture<'static, Result<payload::RefreshFeedPayload, SyndApiError>>;

    fn fetch_feed_status(
        &self,
        url: FeedUrl,
    ) -> BoxFuture<'static, Result<payload::RefreshStatus, SyndApiError>>;

    fn fetch_entries(
        &self,
        after: Option<String>,
        first: i64,
    ) -> BoxFuture<'static, Result<payload::FetchEntriesPayload, SyndApiError>>;

    fn run_timeline_changes(
        &self,
        events: mpsc::UnboundedSender<payload::TimelineChangeEvent>,
    ) -> BoxFuture<'static, Result<(), SyndApiError>>;
}
