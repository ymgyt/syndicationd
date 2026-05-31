use std::future::Future;

use thiserror::Error;

use crate::{
    db::FeedRegistryDb,
    error::RegistryDbError,
    event::{Event, EventInterests, EventKind},
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
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> impl Future<Output = ProcessorResult<RecordedEvents>> + Send;
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
