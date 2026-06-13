use std::future::Future;

use chrono::{DateTime, Utc};
use synd_feed::feed::service::FeedParseError;
use synd_feed::types::FeedUrl;
use thiserror::Error;
use tracing::warn;

use crate::{
    crawl::{
        queue::CrawlJobQueue,
        schedule::{CrawlScheduleCandidate, UpsertCrawlScheduleCommand},
        target_list::{CrawlTarget, FeedEndpointSubscriptionSet},
    },
    db::{CrawlJobQueueTx, CrawlScheduleTx, CrawlTargetTx, FeedRegistryDb, SubscriptionTx},
    entry::EntryProjectionScope,
    error::{RegistryDbError, RegistryDbResult},
    event::{Event, EventInterests, EventPayloadError, EventType, RegistryEvent},
    feed::FeedProjectionScope,
    query::{Subscriptions, SubscriptionsQuery},
    subscription::{
        FeedSubscriptionAttrs, SubscribeOutcome, SubscriberId, SubscriptionKey, UnsubscribeOutcome,
    },
    timeline::TimelineProjectionScope,
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
        actual: EventType,
    },
    #[error(transparent)]
    FeedParse(#[from] FeedParseError),
}

impl From<EventPayloadError> for ProcessorError {
    fn from(err: EventPayloadError) -> Self {
        Self::unexpected_event(err.expected.as_str(), err.actual)
    }
}

impl ProcessorError {
    pub fn unexpected_event(expected: &'static str, actual: EventType) -> Self {
        Self::UnexpectedEvent { expected, actual }
    }

    pub fn unexpected_input(expected: &'static str, event: &Event) -> Self {
        Self::unexpected_event(expected, event.event_type())
    }
}

/// Retry behavior for a processor failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    Retryable,
    Permanent,
}

/// Classifies whether a processor failure should be retried from the same cursor.
pub trait ClassifyError {
    fn classify(&self) -> FailureClass;
}

impl ClassifyError for ProcessorError {
    fn classify(&self) -> FailureClass {
        match self {
            Self::RegistryDb(_) | Self::UnexpectedEvent { .. } | Self::FeedParse(_) => {
                FailureClass::Permanent
            }
        }
    }
}

/// Stable identity for an event processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessorId {
    SubscriptionRequest,
    CrawlTargetProjection,
    FeedProjection,
    EntryProjection,
    TimelineProjection,
    ApiEventProjection,
    ApiEventPublisher,
}

impl ProcessorId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SubscriptionRequest => "SubscriptionRequest",
            Self::CrawlTargetProjection => "CrawlTargetProjection",
            Self::FeedProjection => "FeedProjection",
            Self::EntryProjection => "EntryProjection",
            Self::TimelineProjection => "TimelineProjection",
            Self::ApiEventProjection => "ApiEventProjection",
            Self::ApiEventPublisher => "ApiEventPublisher",
        }
    }
}

pub(crate) fn skip_permanent_error(
    processor: ProcessorId,
    err: ProcessorError,
    context: &'static str,
) -> ProcessorResult<()> {
    match err.classify() {
        FailureClass::Permanent => {
            warn!(
                processor = processor.as_str(),
                context,
                error = %err,
                "registry event processor skipped permanent failure"
            );
            Ok(())
        }
        FailureClass::Retryable => Err(err),
    }
}

/// Typed processor input built from one journaled event.
pub trait ConsumerInput: Sized + Send {
    const INTERESTS: &'static [EventType];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self>;
}

impl<T> ConsumerInput for T
where
    T: RegistryEvent + TryFrom<Event> + Send,
    ProcessorError: From<<T as TryFrom<Event>>::Error>,
{
    const INTERESTS: &'static [EventType] = &[T::TYPE];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        event.try_into().map_err(ProcessorError::from)
    }
}

/// Common declaration for event processors that advance through the journal.
pub trait Processor: Send + 'static {
    type Input: ConsumerInput;

    fn id(&self) -> ProcessorId;

    fn interests(&self) -> EventInterests {
        EventInterests::new(Self::Input::INTERESTS.to_vec())
    }
}

/// Inputs selected from one journal read for a processor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBatch<I> {
    inputs: Vec<I>,
}

impl<I> InputBatch<I> {
    pub fn new(inputs: Vec<I>) -> Self {
        Self { inputs }
    }

    pub fn len(&self) -> usize {
        self.inputs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &I> {
        self.inputs.iter()
    }

    pub fn into_inputs(self) -> Vec<I> {
        self.inputs
    }
}

/// A component that consumes registry events inside a registry transaction.
pub trait Consumer<S>: Processor
where
    S: FeedRegistryDb,
{
    fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> impl Future<Output = ProcessorResult<Vec<Event>>> + Send;

    fn consume_batch(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        batch: InputBatch<Self::Input>,
    ) -> impl Future<Output = ProcessorResult<Vec<Event>>> + Send {
        async move {
            let processor = self.id();
            let mut events = Vec::new();
            for input in batch.into_inputs() {
                match self.consume(cx, input).await {
                    Ok(mut produced) => events.append(&mut produced),
                    Err(err) => skip_permanent_error(processor, err, "input")?,
                }
            }
            Ok(events)
        }
    }
}

/// A terminal event processor that consumes committed events without recording new events.
pub trait Sink: Processor {
    fn consume(&mut self, input: Self::Input) -> impl Future<Output = ProcessorResult<()>> + Send;
}

/// Summary of events recorded into the journal by one submit operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEvents {
    types: Vec<EventType>,
}

impl RecordedEvents {
    pub fn new(types: Vec<EventType>) -> Self {
        Self { types }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self::new(Vec::with_capacity(capacity))
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    pub fn len(&self) -> usize {
        self.types.len()
    }

    pub fn types(&self) -> &[EventType] {
        &self.types
    }

    pub fn push(&mut self, event_type: EventType) {
        self.types.push(event_type);
    }

    pub fn extend(&mut self, mut other: Self) {
        self.types.append(&mut other.types);
    }
}

/// Transactional registry context used by event-driven processors.
///
/// Domain database operations are delegated to the underlying transaction.
/// Processors return produced events to their worker, which records them after
/// domain database writes succeed.
pub struct RegistryContext<'a, Tx> {
    tx: &'a mut Tx,
}

/// Transactional context passed to event consumers.
pub type ConsumeContext<'a, Tx> = RegistryContext<'a, Tx>;

/// Transactional context passed to reconcilers.
pub type ReconcileContext<'a, Tx> = RegistryContext<'a, Tx>;

impl<'a, Tx> RegistryContext<'a, Tx> {
    pub fn new(tx: &'a mut Tx) -> Self {
        Self { tx }
    }

    pub fn subscriber_scope(&mut self, subscriber_id: SubscriberId) -> SubscriberScope<'_, Tx> {
        SubscriberScope::new(&mut *self.tx, subscriber_id)
    }

    /// Returns feed projection operations within this transaction.
    pub fn feed_projection(&mut self) -> FeedProjectionScope<'_, Tx> {
        FeedProjectionScope::new(&mut *self.tx)
    }

    /// Returns entry projection operations within this transaction.
    pub fn entry_projection(&mut self) -> EntryProjectionScope<'_, Tx> {
        EntryProjectionScope::new(&mut *self.tx)
    }

    /// Returns timeline projection operations within this transaction.
    pub fn timeline_projection(&mut self) -> TimelineProjectionScope<'_, Tx> {
        TimelineProjectionScope::new(&mut *self.tx)
    }
}

impl<Tx> RegistryContext<'_, Tx>
where
    Tx: CrawlScheduleTx + Send,
{
    pub async fn list_candidates(
        &mut self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> RegistryDbResult<Vec<CrawlScheduleCandidate>> {
        self.tx.list_candidates(now, limit).await
    }

    pub async fn upsert_schedule(
        &mut self,
        schedule: UpsertCrawlScheduleCommand,
    ) -> RegistryDbResult<()> {
        self.tx.upsert_schedule(schedule).await
    }
}

impl<Tx> RegistryContext<'_, Tx>
where
    Tx: CrawlJobQueueTx + Send,
{
    pub fn crawl_job_queue(&mut self) -> CrawlJobQueue<'_, Tx> {
        CrawlJobQueue::new(&mut *self.tx)
    }
}

/// Subscriber-scoped operations over feed subscription state.
pub struct SubscriberScope<'a, Tx> {
    tx: &'a mut Tx,
    subscriber_id: SubscriberId,
}

impl<'a, Tx> SubscriberScope<'a, Tx> {
    fn new(tx: &'a mut Tx, subscriber_id: SubscriberId) -> Self {
        Self { tx, subscriber_id }
    }
}

impl<Tx> SubscriberScope<'_, Tx>
where
    Tx: SubscriptionTx + Send,
{
    pub async fn subscribe_feed(
        &mut self,
        feed_url: FeedUrl,
        attrs: FeedSubscriptionAttrs,
        now: DateTime<Utc>,
    ) -> ProcessorResult<SubscribeOutcome> {
        let subscription = SubscriptionKey::new(self.subscriber_id.clone(), feed_url);
        let already_subscribed = self
            .tx
            .has_feed_subscription(&subscription.subscriber_id, &subscription.feed_url)
            .await?;

        self.tx
            .upsert_feed_endpoint(&subscription.feed_url, now)
            .await?;
        self.tx
            .upsert_feed_subscription(&subscription, attrs, now)
            .await?;

        let outcome = if already_subscribed {
            SubscribeOutcome::Changed(subscription)
        } else {
            SubscribeOutcome::Subscribed(subscription)
        };
        Ok(outcome)
    }

    pub async fn unsubscribe_feed(
        &mut self,
        feed_url: FeedUrl,
    ) -> ProcessorResult<UnsubscribeOutcome> {
        let subscription = SubscriptionKey::new(self.subscriber_id.clone(), feed_url);
        let is_subscribed = self
            .tx
            .has_feed_subscription(&subscription.subscriber_id, &subscription.feed_url)
            .await?;

        if is_subscribed {
            self.tx
                .delete_feed_subscription(&subscription.subscriber_id, &subscription.feed_url)
                .await?;
            Ok(UnsubscribeOutcome::Unsubscribed(subscription))
        } else {
            Ok(UnsubscribeOutcome::NotSubscribed(subscription))
        }
    }
}

impl<Tx> SubscriptionTx for ConsumeContext<'_, Tx>
where
    Tx: SubscriptionTx + Send,
{
    fn upsert_feed_endpoint(
        &mut self,
        feed_url: &FeedUrl,
        now: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send {
        self.tx.upsert_feed_endpoint(feed_url, now)
    }

    fn upsert_feed_subscription(
        &mut self,
        subscription: &SubscriptionKey,
        attrs: FeedSubscriptionAttrs,
        now: DateTime<Utc>,
    ) -> impl Future<Output = RegistryDbResult<()>> + Send {
        self.tx.upsert_feed_subscription(subscription, attrs, now)
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

    fn load_feed_endpoint_subscriptions(
        &mut self,
        feed_url: &FeedUrl,
    ) -> impl Future<Output = RegistryDbResult<FeedEndpointSubscriptionSet>> + Send {
        self.tx.load_feed_endpoint_subscriptions(feed_url)
    }
}

impl<Tx> CrawlTargetTx for ConsumeContext<'_, Tx>
where
    Tx: CrawlTargetTx + Send,
{
    fn upsert_crawl_target(
        &mut self,
        target: &CrawlTarget,
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
