use std::{collections::HashMap, future::Future};

use chrono::{DateTime, Utc};
use synd_feed::{
    entry::EntryId,
    types::{Feed, FeedUrl},
};

use crate::{
    crawl::{
        blob::{BlobRef, PutBlobCommand},
        due::CrawlDueInput,
        state::{CrawlState, UpsertCrawlStateCommand},
        target_list::{CrawlTarget, FeedSubscriptions},
    },
    entry::Entries,
    error::{RegistryDbError, RegistryDbResult},
    event::{EventJournal, EventJournalAppend},
    feed::FeedUpdate,
    query::{
        Subscriptions, SubscriptionsQuery, TimelineChangesPage, TimelineChangesQuery,
        TimelineEntriesPage, TimelineEntriesQuery,
    },
    subscription::{FeedSubscriptionAttrs, SubscriberId, SubscriptionKey},
    timeline::TimelineCatchup,
};

/// Opens registry database transactions.
pub trait FeedRegistryDb: Clone + Send + Sync + 'static {
    type Tx<'a>: EventJournal + EventJournalAppend + CommitTx + Send
    where
        Self: 'a;

    fn begin(&self) -> impl Future<Output = Result<Self::Tx<'_>, RegistryDbError>> + Send;
}

/// Transactional operations over feed subscription state.
pub trait SubscriptionDb {
    /// Creates or updates the relation. `now` becomes `subscribed_at` on
    /// creation; editing an existing relation keeps the original value.
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

    fn load_feed_subscriptions(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<FeedSubscriptions>> + Send;
}

/// Transactional operations over crawl target state.
pub trait CrawlTargetDb {
    /// Writes the target's declared state. A pending manual request on the
    /// row is preserved: it belongs to the request/completion lifecycle.
    fn upsert_target(
        &mut self,
        target: &CrawlTarget,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn load_target(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlTarget>>> + Send;

    /// Loads the scheduler facts for one active target.
    fn load_crawl_due_input(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlDueInput>>> + Send;

    /// Loads the scheduler facts for every active target.
    fn list_crawl_due_inputs(
        &mut self,
    ) -> impl Future<Output = RegistryDbResult<Vec<CrawlDueInput>>> + Send;

    /// Marks a pending manual crawl request unless one is already pending.
    fn set_manual_request(
        &mut self,
        feed_url: &FeedUrl,
        requested_at: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    /// Clears a pending manual request served by a crawl that started at or
    /// after it. Requests made while the crawl was running stay pending.
    fn clear_manual_request(
        &mut self,
        feed_url: &FeedUrl,
        served_by_crawl_started_at: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;
}

/// Transactional operations over per-feed crawl observation state.
pub trait CrawlStateDb {
    fn load_crawl_state(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlState>>> + Send;

    fn upsert_crawl_state(
        &mut self,
        command: UpsertCrawlStateCommand,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;
}

/// Transactional generic blob-store operations.
pub trait BlobDb {
    fn put_blob(
        &mut self,
        command: PutBlobCommand,
    ) -> impl Future<Output = RegistryDbResult<BlobRef>> + Send;

    fn load_blob(
        &mut self,
        blob: BlobRef,
    ) -> impl Future<Output = RegistryDbResult<Vec<u8>>> + Send;
}

/// Transactional operations over parsed current feed state.
pub trait FeedDb {
    fn load_entries(
        &mut self,
        entry_ids: &[EntryId],
    ) -> impl Future<Output = RegistryDbResult<Entries>> + Send;

    fn apply_feed_update(
        &mut self,
        update: &FeedUpdate,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send;

    fn load_feeds(
        &mut self,
        feed_urls: &[FeedUrl],
    ) -> impl Future<Output = RegistryDbResult<HashMap<FeedUrl, Feed>>> + Send;
}

/// Transactional operations for reading and applying timeline membership.
pub trait TimelineDb {
    fn list_timeline_entries(
        &mut self,
        query: TimelineEntriesQuery,
    ) -> impl Future<Output = RegistryDbResult<TimelineEntriesPage>> + Send;

    fn list_timeline_changes(
        &mut self,
        query: TimelineChangesQuery,
    ) -> impl Future<Output = RegistryDbResult<TimelineChangesPage>> + Send;

    fn catchup_subscribed_feed(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<TimelineCatchup>> + Send;

    /// Applies one entry to the timelines of its subscribers.
    /// `content_changed` marks that the entry content itself changed, which
    /// bumps the item seq so syncing clients re-read the entry.
    fn apply_entry_to_timelines(
        &mut self,
        feed_url: &FeedUrl,
        entry_id: &EntryId,
        content_changed: bool,
    ) -> impl Future<Output = RegistryDbResult<Vec<SubscriberId>>> + Send;

    fn apply_feed_unsubscribed(
        &mut self,
        subscription: &SubscriptionKey,
    ) -> impl Future<Output = RegistryDbResult<Option<SubscriberId>>> + Send;
}

/// Commits a registry database transaction.
pub trait CommitTx {
    fn commit(self) -> impl Future<Output = RegistryDbResult<()>> + Send;
}
