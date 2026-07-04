use std::future::Future;

use chrono::{DateTime, Utc};
use synd_feed::feed::service::FeedParseError;
use thiserror::Error;
use tracing::warn;

use crate::{
    db::FeedRegistryDb,
    error::RegistryDbError,
    event::{Event, EventInterests, EventType, Trigger},
};

/// Result type returned by event processors.
pub type ProcessorResult<T> = Result<T, ProcessorError>;

/// Result of one worker reaction, plus an optional request to be woken later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reaction<T> {
    output: T,
    wake: WakeRequest,
}

impl<T> Reaction<T> {
    pub fn new(output: T, wake: WakeRequest) -> Self {
        Self { output, wake }
    }

    pub fn done(output: T) -> Self {
        Self::new(output, WakeRequest::None)
    }

    pub fn into_parts(self) -> (T, WakeRequest) {
        (self.output, self.wake)
    }
}

/// Request for the event loop to wake a worker at a specific future time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeRequest {
    None,
    At(DateTime<Utc>),
}

impl WakeRequest {
    pub fn at(wake_at: DateTime<Utc>) -> Self {
        Self::At(wake_at)
    }
}

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
    CrawlTargetReconciler,
    FeedProjection,
    EntryProjection,
    TimelineProjection,
    ApiEventProjection,
    ApiEventPublisher,
    CrawlReconciler,
}

impl ProcessorId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CrawlTargetReconciler => "CrawlTargetReconciler",
            Self::FeedProjection => "FeedProjection",
            Self::EntryProjection => "EntryProjection",
            Self::TimelineProjection => "TimelineProjection",
            Self::ApiEventProjection => "ApiEventProjection",
            Self::ApiEventPublisher => "ApiEventPublisher",
            Self::CrawlReconciler => "CrawlReconciler",
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
pub trait EventInput: Sized + Send {
    const INTERESTS: &'static [EventType];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self>;
}

/// Common declaration for components that react to registry events.
pub trait Processor: Send + 'static {
    type Input: EventInput;

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

/// A non-idempotent cursor-driven projection over journaled facts.
pub trait Projector<S>: Processor
where
    S: FeedRegistryDb,
{
    fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> impl Future<Output = ProcessorResult<Vec<Event>>> + Send;

    fn project_batch(
        &mut self,
        tx: &mut S::Tx<'_>,
        batch: InputBatch<Self::Input>,
    ) -> impl Future<Output = ProcessorResult<Vec<Event>>> + Send {
        async move {
            let processor = self.id();
            let mut events = Vec::new();
            for input in batch.into_inputs() {
                match self.project(tx, input).await {
                    Ok(mut produced) => events.append(&mut produced),
                    Err(err) => skip_permanent_error(processor, err, "input")?,
                }
            }
            Ok(events)
        }
    }
}

/// An idempotent reconciler driven by selected journal events.
pub trait Reconciler<S>: Processor
where
    S: FeedRegistryDb,
{
    fn reconcile(
        &mut self,
        tx: &mut S::Tx<'_>,
        now: DateTime<Utc>,
        trigger: Trigger,
        batch: InputBatch<Self::Input>,
    ) -> impl Future<Output = ProcessorResult<Reaction<Vec<Event>>>> + Send;
}

/// Handles one input batch read from the event journal.
pub(crate) trait JournalHandler<S>: Processor
where
    S: FeedRegistryDb,
{
    fn handle_journal_batch(
        &mut self,
        tx: &mut S::Tx<'_>,
        now: DateTime<Utc>,
        trigger: Trigger,
        batch: InputBatch<Self::Input>,
    ) -> impl Future<Output = ProcessorResult<Reaction<Vec<Event>>>> + Send;
}

/// Adapts a projector to the journal handler interface.
pub(crate) struct ProjectorAdapter<P> {
    processor: P,
}

impl<P> ProjectorAdapter<P> {
    pub(crate) fn new(processor: P) -> Self {
        Self { processor }
    }
}

impl<P> Processor for ProjectorAdapter<P>
where
    P: Processor,
{
    type Input = P::Input;

    fn id(&self) -> ProcessorId {
        self.processor.id()
    }

    fn interests(&self) -> EventInterests {
        self.processor.interests()
    }
}

impl<S, P> JournalHandler<S> for ProjectorAdapter<P>
where
    S: FeedRegistryDb,
    P: Projector<S>,
{
    async fn handle_journal_batch(
        &mut self,
        tx: &mut S::Tx<'_>,
        _now: DateTime<Utc>,
        _trigger: Trigger,
        batch: InputBatch<Self::Input>,
    ) -> ProcessorResult<Reaction<Vec<Event>>> {
        self.processor
            .project_batch(tx, batch)
            .await
            .map(Reaction::done)
    }
}

/// Adapts a reconciler to the journal handler interface.
pub(crate) struct ReconcilerAdapter<P> {
    processor: P,
}

impl<P> ReconcilerAdapter<P> {
    pub(crate) fn new(processor: P) -> Self {
        Self { processor }
    }
}

impl<P> Processor for ReconcilerAdapter<P>
where
    P: Processor,
{
    type Input = P::Input;

    fn id(&self) -> ProcessorId {
        self.processor.id()
    }

    fn interests(&self) -> EventInterests {
        self.processor.interests()
    }
}

impl<S, P> JournalHandler<S> for ReconcilerAdapter<P>
where
    S: FeedRegistryDb,
    P: Reconciler<S>,
{
    async fn handle_journal_batch(
        &mut self,
        tx: &mut S::Tx<'_>,
        now: DateTime<Utc>,
        trigger: Trigger,
        batch: InputBatch<Self::Input>,
    ) -> ProcessorResult<Reaction<Vec<Event>>> {
        self.processor.reconcile(tx, now, trigger, batch).await
    }
}

/// A best-effort terminal event processor that consumes committed events without recording new events.
pub trait Sink: Processor {
    fn sink(&mut self, input: Self::Input) -> impl Future<Output = ()> + Send;
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
