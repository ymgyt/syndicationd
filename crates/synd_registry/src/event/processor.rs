use std::future::Future;

use chrono::{DateTime, Utc};
use synd_feed::feed::service::FeedParseError;
use thiserror::Error;
use tracing::warn;

use crate::{
    db::FeedRegistryDb,
    error::RegistryDbError,
    event::{Event, EventInterests, EventType},
};

/// Result type returned by event processors.
pub type ProcessorResult<T> = Result<T, ProcessorError>;

/// Result of one worker reaction: events recorded into the journal plus an
/// optional request to be woken at a specific future time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reaction {
    recorded_events: RecordedEvents,
    wake: WakeRequest,
}

impl Reaction {
    pub fn new(recorded_events: RecordedEvents, wake: WakeRequest) -> Self {
        Self {
            recorded_events,
            wake,
        }
    }

    pub fn done(recorded_events: RecordedEvents) -> Self {
        Self::new(recorded_events, WakeRequest::None)
    }

    pub fn into_parts(self) -> (RecordedEvents, WakeRequest) {
        (self.recorded_events, self.wake)
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
    pub fn unexpected_input(expected: &'static str, event: &Event) -> Self {
        Self::UnexpectedEvent {
            expected,
            actual: event.event_type(),
        }
    }
}

/// Retry behavior for a processor failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// Aborts the batch; the cursor stays put and the pass is retried.
    Retryable,
    /// Skips the offending input; retrying would fail the same way.
    Permanent,
}

/// Classifies whether a processor failure should be retried from the same cursor.
pub trait ClassifyError {
    fn classify(&self) -> FailureClass;
}

impl ClassifyError for ProcessorError {
    fn classify(&self) -> FailureClass {
        match self {
            // Storage failures may be transient (lock contention, I/O); the
            // input must not be lost, so the whole pass is retried.
            Self::RegistryDb(_) => FailureClass::Retryable,
            // Deterministic failures of the input itself: retrying the same
            // event can never succeed.
            Self::UnexpectedEvent { .. } | Self::FeedParse(_) => FailureClass::Permanent,
        }
    }
}

/// Stable identity for an event processor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProcessorId {
    CrawlTargetProjection,
    CrawlScheduleProjection,
    FeedProjection,
    EntryProjection,
    TimelineProjection,
    ApiEventPublisher,
}

impl ProcessorId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CrawlTargetProjection => "CrawlTargetProjection",
            Self::CrawlScheduleProjection => "CrawlScheduleProjection",
            Self::FeedProjection => "FeedProjection",
            Self::EntryProjection => "EntryProjection",
            Self::TimelineProjection => "TimelineProjection",
            Self::ApiEventPublisher => "ApiEventPublisher",
        }
    }
}

impl ProcessorError {
    /// The shared failure policy for per-input processing: permanent
    /// failures are logged and skipped, retryable ones abort the batch so
    /// the pass is retried from the same cursor.
    pub(crate) fn skip_permanent(
        self,
        processor: ProcessorId,
        context: &'static str,
    ) -> ProcessorResult<()> {
        match self.classify() {
            FailureClass::Permanent => {
                warn!(
                    processor = processor.as_str(),
                    context,
                    error = %self,
                    "registry event processor skipped permanent failure"
                );
                Ok(())
            }
            FailureClass::Retryable => Err(self),
        }
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

    pub fn into_inputs(self) -> Vec<I> {
        self.inputs
    }
}

/// A cursor-driven projection over journaled facts.
///
/// Projectors are the only journal consumers: state updates, produced
/// events, and the cursor advance are committed in one transaction, so
/// every selected event is applied exactly once and never reordered.
/// Time-driven convergence belongs to [`crate::event::Reconciler`] instead.
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
                    Err(err) => err.skip_permanent(processor, "input")?,
                }
            }
            Ok(events)
        }
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
}
