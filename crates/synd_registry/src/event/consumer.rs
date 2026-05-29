use std::convert::Infallible;

use thiserror::Error;

use crate::event::{
    EventJournal, EventJournalError, EventReadBatch, EventReadFilter, RegistryEvent,
    RegistryEventKind,
};

pub type EventConsumerResult<T> = Result<T, EventConsumerError>;

#[derive(Debug, Error)]
pub enum EventConsumerError {
    #[error(transparent)]
    Journal(#[from] EventJournalError),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

/// Stable identity for a consumer cursor and consumer lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventConsumerId {
    CrawlTargetListProj,
    EntryTLProj,
    CrawlScheduler,
    FeedRevProj,
    EntryRevProj,
}

impl EventConsumerId {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CrawlTargetListProj => "CrawlTargetListProj",
            Self::EntryTLProj => "EntryTLProj",
            Self::CrawlScheduler => "CrawlScheduler",
            Self::FeedRevProj => "FeedRevProj",
            Self::EntryRevProj => "EntryRevProj",
        }
    }
}

/// Typed input built from the journal entries a consumer is interested in.
pub trait ConsumerEventInput: Sized {
    const READ_FILTER: EventReadFilter;

    fn from_batch(batch: EventReadBatch) -> EventConsumerResult<Option<Self>>;
}

/// A component that consumes registry events through a cursor it owns.
pub trait EventConsumer {
    type Input: ConsumerEventInput;

    fn id(&self) -> EventConsumerId;

    fn read_filter(&self) -> EventReadFilter {
        Self::Input::READ_FILTER
    }

    async fn consume<J>(
        &mut self,
        input: Self::Input,
        session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal;
}

/// Passes a journal batch to one concrete consumer.
pub trait ConsumerDispatch {
    async fn consume<J>(
        self,
        batch: EventReadBatch,
        session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal;
}

impl ConsumerDispatch for Infallible {
    async fn consume<J>(
        self,
        _batch: EventReadBatch,
        _session: &mut EventConsumerSession<'_, J>,
    ) -> EventConsumerResult<()>
    where
        J: EventJournal,
    {
        match self {}
    }
}

/// Summary of events recorded into the journal by one submit operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedEvents {
    kinds: Vec<RegistryEventKind>,
}

impl RecordedEvents {
    pub fn new(kinds: Vec<RegistryEventKind>) -> Self {
        Self { kinds }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    pub fn kinds(&self) -> &[RegistryEventKind] {
        &self.kinds
    }

    pub fn push(&mut self, kind: RegistryEventKind) {
        self.kinds.push(kind);
    }

    pub fn extend(&mut self, mut other: Self) {
        self.kinds.append(&mut other.kinds);
    }
}

/// Journal access granted to one consumer while processing one batch.
pub struct EventConsumerSession<'a, J> {
    journal: &'a J,
    recorded: RecordedEvents,
}

impl<'a, J> EventConsumerSession<'a, J>
where
    J: EventJournal,
{
    pub fn new(journal: &'a J) -> Self {
        Self {
            journal,
            recorded: RecordedEvents::empty(),
        }
    }

    pub async fn record(&mut self, event: RegistryEvent) -> EventConsumerResult<()> {
        let kind = event.kind();
        self.journal.append(event).await?;
        self.recorded.push(kind);
        Ok(())
    }

    pub fn recorded(&self) -> &RecordedEvents {
        &self.recorded
    }

    pub fn into_recorded(self) -> RecordedEvents {
        self.recorded
    }
}

/// Runtime-facing registry of concrete event consumers.
pub trait ConsumerRegistry: Clone + Send + Sync + 'static {
    type Dispatch<'a>: ConsumerDispatch + 'a
    where
        Self: 'a;

    fn ids(&self) -> &'static [EventConsumerId];

    fn read_filter(&self, id: EventConsumerId) -> Option<EventReadFilter>;

    fn dispatch(&self, id: EventConsumerId) -> Option<Self::Dispatch<'_>>;

    fn interested_in(&self, recorded: &RecordedEvents) -> Vec<EventConsumerId> {
        let mut consumers = Vec::new();
        for id in self.ids().iter().copied() {
            let Some(filter) = self.read_filter(id) else {
                continue;
            };
            if filter.matches_any(recorded.kinds()) {
                consumers.push(id);
            }
        }
        consumers
    }
}

/// A consumer registry with no registered consumers.
///
/// This is useful while event recording is wired in before projections exist.
#[derive(Debug, Default, Clone, Copy)]
pub struct EmptyConsumerRegistry;

impl ConsumerRegistry for EmptyConsumerRegistry {
    type Dispatch<'a> = Infallible;

    fn ids(&self) -> &'static [EventConsumerId] {
        &[]
    }

    fn read_filter(&self, _id: EventConsumerId) -> Option<EventReadFilter> {
        None
    }

    fn dispatch(&self, _id: EventConsumerId) -> Option<Self::Dispatch<'_>> {
        None
    }
}
