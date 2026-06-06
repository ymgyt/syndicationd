use std::future::Future;

use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::{
    crawl::{
        job::{CrawlQueueSnapshot, EnqueueJob, EnqueueJobResult},
        schedule::{CrawlScheduleCandidate, UpsertSchedule},
        target_list::{CrawlTarget, FeedEndpointSubscriptionSet},
    },
    error::{RegistryDbError, RegistryDbResult},
    event::JournalTx,
    query::{Subscriptions, SubscriptionsQuery},
    subscription::{FeedSubscriptionAttrs, SubscriberId, SubscriptionKey},
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
        now: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn upsert_feed_subscription(
        &mut self,
        subscription: &SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
        now: DateTime<Utc>,
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

    fn load_feed_endpoint_subscriptions(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<FeedEndpointSubscriptionSet>> + Send;

    fn upsert_crawl_target(
        &mut self,
        target: &CrawlTarget,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn load_crawl_target_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlTarget>>> + Send;
}

/// Transactional scheduler-state operations.
pub trait CrawlScheduleTx {
    fn list_candidates(
        &mut self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> impl Future<Output = RegistryDbResult<Vec<CrawlScheduleCandidate>>> + Send;

    fn upsert_schedule(
        &mut self,
        schedule: UpsertSchedule,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;
}

/// Transactional crawl-job queue operations.
pub trait CrawlJobQueueTx {
    fn queue_snapshot(
        &mut self,
    ) -> impl Future<Output = RegistryDbResult<CrawlQueueSnapshot>> + Send;

    fn enqueue_job(
        &mut self,
        job: EnqueueJob,
    ) -> impl Future<Output = RegistryDbResult<EnqueueJobResult>> + Send;
}

/// Commits a registry database transaction.
pub trait CommitTx {
    fn commit(self) -> impl Future<Output = RegistryDbResult<()>> + Send;
}
