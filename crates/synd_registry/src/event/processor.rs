use std::future::Future;

use synd_feed::types::FeedUrl;
use thiserror::Error;

use crate::{
    crawl::target_list::CrawlTarget,
    db::{FeedRegistryDb, RegistryTx},
    error::{RegistryDbError, RegistryDbResult},
    event::{Event, EventInterests, EventKind, JournalTx},
    query::{Subscriptions, SubscriptionsQuery},
    subscription::{SubscriberId, Subscription},
};

/// Result type returned by event processors.
pub type ProcessorResult<T> = Result<T, ProcessorError>;

/// Error returned while converting or processing registry events.
#[derive(Debug, Error)]
pub enum ProcessorError {
    #[error(transparent)]
    RegistryDb(#[from] RegistryDbError),
    #[error("unexpected event for {expected}: {actual:?}")]
    UnexpectedEvent {
        expected: &'static str,
        actual: EventKind,
    },
}

/// Stable identity for an event processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessorId {
    SubscriptionRequest,
    CrawlTargetProjection,
    ApiEventProjection,
    ApiEventPublisher,
}

impl ProcessorId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SubscriptionRequest => "SubscriptionRequest",
            Self::CrawlTargetProjection => "CrawlTargetProjection",
            Self::ApiEventProjection => "ApiEventProjection",
            Self::ApiEventPublisher => "ApiEventPublisher",
        }
    }
}

/// Typed input built from one event a processor is interested in.
pub trait ProcessorInput: TryFrom<Event, Error = ProcessorError> + Send {}

impl<T> ProcessorInput for T where T: TryFrom<Event, Error = ProcessorError> + Send {}

/// Marker for how an event processor participates in transaction boundaries.
pub trait ProcessorPhase: Send + Sync + 'static {}

/// Processor phase that runs inside the registry database transaction.
#[derive(Debug, Clone, Copy)]
pub struct Transactional;

/// Processor phase that runs only after cursor progress is committed.
#[derive(Debug, Clone, Copy)]
pub struct PostCommit;

impl ProcessorPhase for Transactional {}

impl ProcessorPhase for PostCommit {}

/// Common declaration for event processors that advance through the journal.
pub trait Processor: Send + 'static {
    type Input: ProcessorInput;
    type Phase: ProcessorPhase;

    fn id(&self) -> ProcessorId;

    fn interests(&self) -> EventInterests;
}

/// A component that consumes registry events inside a registry transaction.
pub trait Consumer<S>: Processor<Phase = Transactional>
where
    S: FeedRegistryDb,
{
    fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> impl Future<Output = ProcessorResult<()>> + Send;
}

/// A terminal event processor that consumes committed events without recording new events.
pub trait Sink: Processor<Phase = PostCommit> {
    fn consume(&mut self, input: Self::Input) -> impl Future<Output = ProcessorResult<()>> + Send;
}

/// Summary of events recorded into the journal by one submit operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEvents {
    kinds: Vec<EventKind>,
}

impl RecordedEvents {
    pub fn new(kinds: Vec<EventKind>) -> Self {
        Self { kinds }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::new(Vec::with_capacity(capacity))
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    pub fn kinds(&self) -> &[EventKind] {
        &self.kinds
    }

    pub fn push(&mut self, kind: EventKind) {
        self.kinds.push(kind);
    }

    pub fn extend(&mut self, mut other: Self) {
        self.kinds.append(&mut other.kinds);
    }
}

/// Transactional context passed to event consumers.
///
/// Domain database operations are delegated to the underlying transaction.
/// New journal events must be recorded through `record_event` so journal writes
/// and wake summaries stay in sync.
pub struct ConsumeContext<'a, Tx> {
    tx: &'a mut Tx,
    recorded: RecordedEvents,
}

impl<'a, Tx> ConsumeContext<'a, Tx> {
    pub fn new(tx: &'a mut Tx) -> Self {
        Self::with_capacity(tx, 0)
    }

    pub fn with_capacity(tx: &'a mut Tx, capacity: usize) -> Self {
        Self {
            tx,
            recorded: RecordedEvents::with_capacity(capacity),
        }
    }

    pub fn into_recorded(self) -> RecordedEvents {
        self.recorded
    }
}

impl<Tx> ConsumeContext<'_, Tx>
where
    Tx: JournalTx + Send,
{
    pub async fn record_event(&mut self, event: Event) -> ProcessorResult<()> {
        let kind = event.kind();
        self.tx.append_event(event).await?;
        self.recorded.push(kind);
        Ok(())
    }
}

impl<Tx> RegistryTx for ConsumeContext<'_, Tx>
where
    Tx: RegistryTx + Send,
{
    fn upsert_feed_endpoint(
        &mut self,
        feed_url: &FeedUrl,
        now: chrono::DateTime<chrono::Utc>,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send {
        self.tx.upsert_feed_endpoint(feed_url, now)
    }

    fn upsert_feed_subscription(
        &mut self,
        subscription: Subscription,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send {
        self.tx.upsert_feed_subscription(subscription)
    }

    fn delete_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send {
        self.tx.delete_feed_subscription(subscriber_id, feed_url)
    }

    fn has_feed_subscription(
        &mut self,
        subscriber_id: &SubscriberId,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<bool>> + Send {
        self.tx.has_feed_subscription(subscriber_id, feed_url)
    }

    fn list_subscriptions(
        &mut self,
        query: SubscriptionsQuery,
    ) -> impl Future<Output = RegistryDbResult<Subscriptions>> + Send {
        self.tx.list_subscriptions(query)
    }

    fn list_active_subscriptions_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Vec<Subscription>>> + Send {
        self.tx.list_active_subscriptions_for_endpoint(feed_url)
    }

    fn upsert_crawl_target(
        &mut self,
        target: CrawlTarget,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send {
        self.tx.upsert_crawl_target(target)
    }

    fn load_crawl_target_for_endpoint(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<Option<CrawlTarget>>> + Send {
        self.tx.load_crawl_target_for_endpoint(feed_url)
    }
}
