use std::sync::RwLock;

use futures_util::{FutureExt as _, future::BoxFuture};
use synd_client::{
    ApiCredential, Client, FeedEventWatch as ClientFeedEventWatch, SyndApiError, payload,
};
use synd_feed::types::FeedUrl;

use super::{FeedApi, FeedEventWatch};

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

impl FeedEventWatch for ClientFeedEventWatch {
    fn next_event(&mut self) -> BoxFuture<'_, Result<payload::FeedEvent, SyndApiError>> {
        async move { ClientFeedEventWatch::next_event(self).await }.boxed()
    }
}

impl FeedApi for ClientFeedApi {
    fn set_credential(&self, credential: ApiCredential) -> Result<(), SyndApiError> {
        self.client
            .write()
            .expect("feed API client lock must not be poisoned")
            .set_credential(credential)
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

    fn watch_feed_events(
        &self,
    ) -> BoxFuture<'static, Result<Box<dyn FeedEventWatch>, SyndApiError>> {
        let client = self.client();
        async move {
            client
                .watch_feed_events()
                .await
                .map(|watch| Box::new(watch) as Box<dyn FeedEventWatch>)
        }
        .boxed()
    }
}
