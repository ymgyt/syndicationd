use std::sync::RwLock;

use futures_util::{FutureExt as _, future::BoxFuture};
use synd_client::{ApiCredential, Client, SyndApiError, payload};
use synd_feed::types::FeedUrl;
use tokio::sync::mpsc;

use super::FeedApi;

/// Production `FeedApi` adapter backed by `synd_client::Client`.
pub struct ClientFeedApi {
    client: RwLock<Client>,
}

impl ClientFeedApi {
    pub fn new(client: Client) -> Self {
        Self {
            client: RwLock::new(client),
        }
    }

    fn client(&self) -> Client {
        self.client
            .read()
            .expect("feed API client lock must not be poisoned")
            .clone()
    }
}

impl FeedApi for ClientFeedApi {
    fn set_credential(&self, credential: ApiCredential) -> Result<(), SyndApiError> {
        self.client
            .write()
            .expect("feed API client lock must not be poisoned")
            .set_credential(credential)
    }

    fn fetch_initial_feed_view(
        &self,
        subscriptions_first: i64,
        timeline_first: i64,
    ) -> BoxFuture<'static, Result<payload::InitialFeedViewPayload, SyndApiError>> {
        let client = self.client();
        async move {
            client
                .fetch_initial_feed_view(subscriptions_first, timeline_first)
                .await
        }
        .boxed()
    }

    fn fetch_subscription(
        &self,
        after: Option<String>,
        first: Option<i64>,
    ) -> BoxFuture<'static, Result<payload::SubscriptionPayload, SyndApiError>> {
        let client = self.client();
        async move { client.fetch_subscription(after, first).await }.boxed()
    }

    fn subscribe_feed(
        &self,
        input: payload::SubscribeFeedInput,
    ) -> BoxFuture<'static, Result<payload::SubscribeFeedPayload, SyndApiError>> {
        let client = self.client();
        async move { client.subscribe_feed(input).await }.boxed()
    }

    fn unsubscribe_feed(&self, url: FeedUrl) -> BoxFuture<'static, Result<(), SyndApiError>> {
        let client = self.client();
        async move { client.unsubscribe_feed(url).await.map(|_| ()) }.boxed()
    }

    fn refresh_feed(
        &self,
        url: FeedUrl,
    ) -> BoxFuture<'static, Result<payload::RefreshFeedPayload, SyndApiError>> {
        let client = self.client();
        async move { client.refresh_feed(url).await }.boxed()
    }

    fn fetch_feed_status(
        &self,
        url: FeedUrl,
    ) -> BoxFuture<'static, Result<payload::RefreshStatus, SyndApiError>> {
        let client = self.client();
        async move { client.fetch_feed_status(url).await }.boxed()
    }

    fn fetch_timeline_entries(
        &self,
        after: Option<String>,
        first: i64,
    ) -> BoxFuture<'static, Result<payload::TimelineEntryConnection, SyndApiError>> {
        let client = self.client();
        async move { client.fetch_timeline_entries(after, first).await }.boxed()
    }

    fn fetch_timeline_changes(
        &self,
        since: i64,
        first: i64,
    ) -> BoxFuture<'static, Result<payload::TimelineChangesPayload, SyndApiError>> {
        let client = self.client();
        async move { client.fetch_timeline_changes(since, first).await }.boxed()
    }

    fn run_feed_events(
        &self,
        events: mpsc::UnboundedSender<payload::FeedEvent>,
    ) -> BoxFuture<'static, Result<(), SyndApiError>> {
        let client = self.client();
        async move { client.run_feed_events(events).await }.boxed()
    }
}
