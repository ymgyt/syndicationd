use std::future::Future;

use synd_feed::types::FeedUrl;

use crate::{
    crawl::state::{FeedSnapshot, RefreshFailure, RefreshStarted, RefreshState, RefreshSuccess},
    error::{RegistryDbError, RegistryDbResult},
    event::Event,
    subscriber::SubscriberId,
    subscription::Subscription,
    view::{Subscriptions, SubscriptionsQuery},
};

pub trait FeedRegistryDb: Clone + Send + Sync + 'static {
    type Tx<'a>: RegistryDbTransaction + Send
    where
        Self: 'a;

    fn begin(&self) -> impl Future<Output = Result<Self::Tx<'_>, RegistryDbError>> + Send;
}

pub trait RegistryDbTransaction {
    fn append_event(&mut self, event: Event) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn upsert_subscription(
        &mut self,
        subscription: Subscription,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn delete_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn has_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<bool>> + Send;

    fn list_subscriptions(
        &mut self,
        query: SubscriptionsQuery,
    ) -> impl Future<Output = RegistryDbResult<Subscriptions>> + Send;

    fn list_active_subscriptions(
        &mut self,
    ) -> impl Future<Output = RegistryDbResult<Vec<Subscription>>> + Send;

    fn list_active_subscriptions_for_feed(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Vec<Subscription>>> + Send;

    fn list_subscriptions_for_subscriber(
        &mut self,
        subscriber_id: &SubscriberId,
    ) -> impl Future<Output = RegistryDbResult<Vec<Subscription>>> + Send;

    fn list_active_feed_urls(
        &mut self,
    ) -> impl Future<Output = RegistryDbResult<Vec<FeedUrl>>> + Send;

    fn load_snapshots(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> impl Future<Output = RegistryDbResult<Vec<FeedSnapshot>>> + Send;

    fn load_refresh_states(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> impl Future<Output = RegistryDbResult<Vec<RefreshState>>> + Send;

    fn delete_feed_state(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn record_refresh_started(
        &mut self,
        event: RefreshStarted,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn record_refresh_succeeded(
        &mut self,
        result: RefreshSuccess,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn record_refresh_failed(
        &mut self,
        result: RefreshFailure,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn commit(self) -> impl Future<Output = RegistryDbResult<()>> + Send;
}
