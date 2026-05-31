use std::future::Future;

use thiserror::Error;

use crate::{
    error::RegistryDbError,
    event::{EventJournalError, EventKind, EventReadBatch, EventReadFilter},
};

pub type EventConsumerResult<T> = Result<T, EventConsumerError>;

#[derive(Debug, Error)]
pub enum EventConsumerError {
    #[error(transparent)]
    Journal(#[from] EventJournalError),
    #[error(transparent)]
    RegistryDb(#[from] RegistryDbError),
}

/// Stable identity for a consumer cursor and consumer lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventConsumerId {
    SubRequestWorker,
    CrawlTargetListProj,
    ApiEventProj,
    ApiEventStream,
    EntryTLProj,
    CrawlScheduler,
    FeedRevProj,
    EntryRevProj,
}

impl EventConsumerId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SubRequestWorker => "SubRequestWorker",
            Self::CrawlTargetListProj => "CrawlTargetListProj",
            Self::ApiEventProj => "ApiEventProj",
            Self::ApiEventStream => "ApiEventStream",
            Self::EntryTLProj => "EntryTLProj",
            Self::CrawlScheduler => "CrawlScheduler",
            Self::FeedRevProj => "FeedRevProj",
            Self::EntryRevProj => "EntryRevProj",
        }
    }
}

/// Typed input built from the journal entries a consumer is interested in.
pub trait ConsumerEventInput: Sized + Send {
    const READ_FILTER: EventReadFilter;

    fn from_batch(batch: EventReadBatch) -> EventConsumerResult<Option<Self>>;
}

/// A component that consumes registry events through a cursor it owns.
pub trait EventConsumer: Send + 'static {
    type Input: ConsumerEventInput;

    fn id(&self) -> EventConsumerId;

    fn read_filter(&self) -> EventReadFilter {
        Self::Input::READ_FILTER
    }

    fn consume(
        &mut self,
        input: Self::Input,
    ) -> impl Future<Output = EventConsumerResult<RecordedEvents>> + Send;
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
