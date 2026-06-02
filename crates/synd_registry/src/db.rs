use std::future::Future;

use synd_feed::types::FeedUrl;

use crate::{
    crawl::target_list::CrawlTarget,
    error::{RegistryDbError, RegistryDbResult},
    event::JournalTx,
    query::{Subscriptions, SubscriptionsQuery},
    subscription::{SubscriberId, Subscription},
};

/// Opens registry database transactions.
pub trait FeedRegistryDb: Clone + Send + Sync + 'static {
    type Tx<'a>: RegistryTx + JournalTx + CommitTx + Send
    where
        Self: 'a;

    fn begin(&self) -> impl Future<Output = Result<Self::Tx<'_>, RegistryDbError>> + Send;
}

/// Transactional registry-domain operations.
pub trait RegistryTx {
    fn upsert_feed_endpoint(
        &mut self,
        feed_url: &FeedUrl,
        now: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn upsert_feed_subscription(
        &mut self,
        subscription: Subscription,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn delete_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn has_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<bool>> + Send;

    fn list_subscriptions(
        &mut self,
        query: SubscriptionsQuery,
    ) -> impl Future<Output = RegistryDbResult<Subscriptions>> + Send;

    fn list_active_subscriptions_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Vec<Subscription>>> + Send;

    fn upsert_crawl_target(
        &mut self,
        target: CrawlTarget,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn load_crawl_target_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlTarget>>> + Send;
}

/// Commits a registry database transaction.
pub trait CommitTx {
    fn commit(self) -> impl Future<Output = RegistryDbResult<()>> + Send;
}
