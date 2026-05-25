use synd_feed::types::FeedUrl;

use crate::{
    error::{StoreError, StoreResult},
    model::{
        FeedSnapshot, FeedSubscription, FeedSubscriptionPage, ListSubscriptionsQuery,
        RefreshFailure, RefreshStarted, RefreshState, RefreshSuccess, SubscriberId,
    },
};

pub trait FeedRegistryStore: Clone + Send + Sync + 'static {
    type Tx<'a>: RegistryTransaction + Send
    where
        Self: 'a;

    async fn begin(&self) -> Result<Self::Tx<'_>, StoreError>;
}

pub trait RegistryTransaction {
    async fn upsert_subscription(&mut self, subscription: FeedSubscription) -> StoreResult<()>;

    async fn delete_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> StoreResult<()>;

    async fn has_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> StoreResult<bool>;

    async fn list_subscriptions(
        &mut self,
        query: ListSubscriptionsQuery,
    ) -> StoreResult<FeedSubscriptionPage>;

    async fn list_active_subscriptions(&mut self) -> StoreResult<Vec<FeedSubscription>>;

    async fn list_active_subscriptions_for_feed(
        &mut self,
        feed_url: &FeedUrl,
    ) -> StoreResult<Vec<FeedSubscription>>;

    async fn list_subscriptions_for_subscriber(
        &mut self,
        subscriber_id: &SubscriberId,
    ) -> StoreResult<Vec<FeedSubscription>>;

    async fn list_active_feed_urls(&mut self) -> StoreResult<Vec<FeedUrl>>;

    async fn load_snapshots(&mut self, feed_urls: &[FeedUrl]) -> StoreResult<Vec<FeedSnapshot>>;

    async fn load_refresh_states(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> StoreResult<Vec<RefreshState>>;

    async fn delete_feed_state(&mut self, feed_url: &FeedUrl) -> StoreResult<()>;

    async fn record_refresh_started(&mut self, event: RefreshStarted) -> StoreResult<()>;

    async fn record_refresh_succeeded(&mut self, result: RefreshSuccess) -> StoreResult<()>;

    async fn record_refresh_failed(&mut self, result: RefreshFailure) -> StoreResult<()>;

    async fn commit(self) -> StoreResult<()>;
}
