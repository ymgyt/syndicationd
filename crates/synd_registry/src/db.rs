use std::future::Future;

use chrono::{DateTime, Utc};
use synd_feed::types::{EntryId, FeedUrl};

use crate::{
    crawl::{
        blob::{BlobRef, PutBlobCommand},
        job::{
            ClaimCrawlJobCommand, ClaimCrawlJobOutcome, CrawlJobId, EnqueueCrawlJobCommand,
            EnqueueCrawlJobOutcome, FinishCrawlJobCommand, FinishCrawlJobOutcome,
        },
        result::{CrawlResultRef, CrawlState, RecordCrawlResultCommand, UpsertCrawlStateCommand},
        schedule::{CrawlScheduleCandidate, UpsertCrawlScheduleCommand},
        target_list::{CrawlTarget, FeedEndpointSubscriptionSet},
    },
    entry::{EntryChanges, EntrySet},
    error::{RegistryDbError, RegistryDbResult},
    event::JournalTx,
    feed::{FeedSource, UpsertFeedCommand, UpsertFeedOutcome},
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
    subscription::{FeedSubscriptionAttrs, SubscriberId, SubscriptionKey},
    timeline::{TimelineCatchup, TimelineKey},
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

    fn list_timeline_items(
        &mut self,
        query: TimelineItemsQuery,
    ) -> impl Future<Output = RegistryDbResult<TimelineItemsPage>> + Send;

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
        command: UpsertCrawlScheduleCommand,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;
}

/// Transactional crawl-job queue operations.
pub trait CrawlJobQueueTx {
    fn enqueue_job(
        &mut self,
        command: EnqueueCrawlJobCommand,
    ) -> impl Future<Output = RegistryDbResult<EnqueueCrawlJobOutcome>> + Send;

    fn claim_job(
        &mut self,
        command: ClaimCrawlJobCommand,
    ) -> impl Future<Output = RegistryDbResult<ClaimCrawlJobOutcome>> + Send;

    fn finish_job(
        &mut self,
        command: FinishCrawlJobCommand,
    ) -> impl Future<Output = RegistryDbResult<FinishCrawlJobOutcome>> + Send;
}

/// Transactional generic blob-store operations.
pub trait BlobStoreTx {
    fn put_blob(
        &mut self,
        command: PutBlobCommand,
    ) -> impl Future<Output = RegistryDbResult<BlobRef>> + Send;

    fn load_blob(
        &mut self,
        blob: BlobRef,
    ) -> impl Future<Output = RegistryDbResult<Vec<u8>>> + Send;
}

/// Transactional operations for persisting one crawl job completion.
pub trait CrawlCompletionTx {
    fn load_crawl_state(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlState>>> + Send;

    fn record_crawl_result(
        &mut self,
        command: RecordCrawlResultCommand,
    ) -> impl Future<Output = RegistryDbResult<CrawlResultRef>> + Send;

    fn upsert_crawl_state(
        &mut self,
        command: UpsertCrawlStateCommand,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;
}

/// Transactional operations for applying parsed feed state to the registry.
pub trait FeedProjectionTx {
    fn load_feed_source(
        &mut self,
        job_id: &CrawlJobId,
    ) -> impl Future<Output = RegistryDbResult<Option<FeedSource>>> + Send;

    fn upsert_feed(
        &mut self,
        command: UpsertFeedCommand,
    ) -> impl Future<Output = RegistryDbResult<UpsertFeedOutcome>> + Send;
}

/// Transactional operations for applying reconciled entry state.
pub trait EntryProjectionTx {
    fn load_entry_source(
        &mut self,
        job_id: &CrawlJobId,
    ) -> impl Future<Output = RegistryDbResult<Option<FeedSource>>> + Send;

    fn load_entries(
        &mut self,
        feed_url: &FeedUrl,
        entry_ids: &[EntryId],
    ) -> impl Future<Output = RegistryDbResult<EntrySet>> + Send;

    fn apply_entry_changes(
        &mut self,
        changes: EntryChanges,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;
}

/// Transactional operations for applying timeline membership.
pub trait TimelineProjectionTx {
    fn ensure_default_timeline(
        &mut self,
        timeline: &TimelineKey,
        now: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn catchup_timeline_feed(
        &mut self,
        timeline: &TimelineKey,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<TimelineCatchup>> + Send;

    fn apply_entry_discovered(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        now: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<Vec<TimelineKey>>> + Send;

    fn apply_entry_changed(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        now: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<Vec<TimelineKey>>> + Send;

    fn apply_feed_unsubscribed(
        &mut self,
        subscription: &SubscriptionKey,
    ) -> impl Future<Output = RegistryDbResult<Option<TimelineKey>>> + Send;
}

/// Commits a registry database transaction.
pub trait CommitTx {
    fn commit(self) -> impl Future<Output = RegistryDbResult<()>> + Send;
}
