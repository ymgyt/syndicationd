use std::future::Future;

use synd_feed::types::FeedUrl;

use crate::{
    crawl::target_list::CrawlTarget,
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

    fn list_active_subscriptions_for_feed(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Vec<Subscription>>> + Send;

    fn upsert_crawl_target(
        &mut self,
        target: CrawlTarget,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn load_crawl_target(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlTarget>>> + Send;

    fn commit(self) -> impl Future<Output = RegistryDbResult<()>> + Send;
}
