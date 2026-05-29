use synd_feed::types::FeedUrl;

use crate::{
    error::{RegistryDbError, RegistryDbResult},
    event::RegistryEvent,
    legacy::model::{
        FeedSnapshot, FeedSubscription, FeedSubscriptionPage, ListSubscriptionsQuery,
        RefreshFailure, RefreshStarted, RefreshState, RefreshSuccess, SubscriberId,
    },
};

pub trait FeedRegistryDb: Clone + Send + Sync + 'static {
    type Tx<'a>: RegistryDbTransaction + Send
    where
        Self: 'a;

    async fn begin(&self) -> Result<Self::Tx<'_>, RegistryDbError>;
}

pub trait RegistryDbTransaction {
    async fn append_event(&mut self, event: RegistryEvent) -> RegistryDbResult<()>;

    async fn upsert_subscription(&mut self, subscription: FeedSubscription)
    -> RegistryDbResult<()>;

    async fn delete_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<()>;

    async fn has_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<bool>;

    async fn list_subscriptions(
        &mut self,
        query: ListSubscriptionsQuery,
    ) -> RegistryDbResult<FeedSubscriptionPage>;

    async fn list_active_subscriptions(&mut self) -> RegistryDbResult<Vec<FeedSubscription>>;

    async fn list_active_subscriptions_for_feed(
        &mut self,
        feed_url: &FeedUrl,
    ) -> RegistryDbResult<Vec<FeedSubscription>>;

    async fn list_subscriptions_for_subscriber(
        &mut self,
        subscriber_id: &SubscriberId,
    ) -> RegistryDbResult<Vec<FeedSubscription>>;

    async fn list_active_feed_urls(&mut self) -> RegistryDbResult<Vec<FeedUrl>>;

    async fn load_snapshots(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> RegistryDbResult<Vec<FeedSnapshot>>;

    async fn load_refresh_states(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> RegistryDbResult<Vec<RefreshState>>;

    async fn delete_feed_state(&mut self, feed_url: &FeedUrl) -> RegistryDbResult<()>;

    async fn record_refresh_started(&mut self, event: RefreshStarted) -> RegistryDbResult<()>;

    async fn record_refresh_succeeded(&mut self, result: RefreshSuccess) -> RegistryDbResult<()>;

    async fn record_refresh_failed(&mut self, result: RefreshFailure) -> RegistryDbResult<()>;

    async fn commit(self) -> RegistryDbResult<()>;
}
