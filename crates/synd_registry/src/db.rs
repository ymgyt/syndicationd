use std::future::Future;

use chrono::{DateTime, Utc};
use synd_feed::types::{EntryId, FeedUrl};

use crate::{
    crawl::{
        blob::{BlobRef, PutBlobCommand},
        job::CrawlJobId,
        result::{CrawlResultRef, CrawlState, RecordCrawlResultCommand, UpsertCrawlStateCommand},
        schedule::{ScheduleSyncEntry, ScheduledDue, UpsertCrawlScheduleCommand},
        target_list::{CrawlTarget, FeedEndpointSubscriptionSet},
    },
    entry::{EntryChanges, EntrySet},
    error::{RegistryDbError, RegistryDbResult},
    event::{EventJournal, EventJournalAppend},
    feed::{FeedSource, UpsertFeedCommand, UpsertFeedOutcome},
    query::{Subscriptions, SubscriptionsQuery, TimelineItemsPage, TimelineItemsQuery},
    subscription::{FeedSubscriptionAttrs, SubscriberId, SubscriptionKey},
    timeline::{TimelineCatchup, TimelineKey},
};

/// Opens registry database transactions.
pub trait FeedRegistryDb: Clone + Send + Sync + 'static {
    type Tx<'a>: EventJournal + EventJournalAppend + CommitTx + Send
    where
        Self: 'a;

    fn begin(&self) -> impl Future<Output = Result<Self::Tx<'_>, RegistryDbError>> + Send;
}

/// Transactional operations over feed subscription state.
pub trait SubscriptionStore {
    fn upsert_subscription(
        &mut self,
        subscription: &SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
        now: DateTime<Utc>,
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

    fn load_endpoint_subscriptions(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<FeedEndpointSubscriptionSet>> + Send;
}

/// Transactional operations over crawl target state.
pub trait CrawlTargetStore {
    fn upsert_target(
        &mut self,
        target: &CrawlTarget,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn load_target_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlTarget>>> + Send;
}

/// Transactional scheduler-state operations.
pub trait CrawlScheduleStore {
    fn load_schedule_sync_entry(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<ScheduleSyncEntry>>> + Send;

    fn list_schedule_sync_entries(
        &mut self,
        limit: usize,
    ) -> impl Future<Output = RegistryDbResult<Vec<ScheduleSyncEntry>>> + Send;

    fn list_scheduled_due(
        &mut self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> impl Future<Output = RegistryDbResult<Vec<ScheduledDue>>> + Send;

    fn upsert_schedule(
        &mut self,
        command: UpsertCrawlScheduleCommand,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;
}

/// Transactional generic blob-store operations.
pub trait BlobStore {
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
pub trait CrawlResultStore {
    fn load_crawl_source(
        &mut self,
        job_id: &CrawlJobId,
    ) -> impl Future<Output = RegistryDbResult<Option<FeedSource>>> + Send;

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
pub trait FeedStore {
    fn upsert_feed(
        &mut self,
        command: UpsertFeedCommand,
    ) -> impl Future<Output = RegistryDbResult<UpsertFeedOutcome>> + Send;
}

/// Transactional operations for applying reconciled entry state.
pub trait EntryStore {
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

/// Transactional operations for reading and applying timeline membership.
pub trait TimelineStore {
    fn list_timeline_items(
        &mut self,
        query: TimelineItemsQuery,
    ) -> impl Future<Output = RegistryDbResult<TimelineItemsPage>> + Send;

    fn catchup_subscribed_feed(
        &mut self,
        timeline: &TimelineKey,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<TimelineCatchup>> + Send;

    fn apply_entry_to_timelines(
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
