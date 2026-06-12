use anyhow::anyhow;
use bon::Builder;
use synd_feed::feed::service::FeedService;
use synd_feed::types::FeedUrl;

use crate::{
    crawl::job::CrawlJobId,
    db::{BlobStoreTx, EntryProjectionTx, FeedRegistryDb},
    entry::{EntryAppearances, EntryChange, EntryChanges, EntryReconciliation, EntrySet},
    error::{RegistryDbError, RegistryDbResult},
    event::{
        ConsumeContext, Consumer, EntryChangedEvent, EntryDiscoveredEvent, Event, EventInterests,
        FeedChangedEvent, FeedDiscoveredEvent, FeedEvent, FeedEventKind, JournalTx, Processor,
        ProcessorError, ProcessorId, ProcessorResult, RecordedEvents, Transactional,
    },
    feed::FeedSource,
};

/// Event input used to project entry state.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct EntryProjectionInput {
    pub feed_url: FeedUrl,
    pub crawl_job_id: CrawlJobId,
}

impl From<FeedDiscoveredEvent> for EntryProjectionInput {
    fn from(event: FeedDiscoveredEvent) -> Self {
        Self::builder()
            .feed_url(event.feed_url)
            .crawl_job_id(event.crawl_job_id)
            .build()
    }
}

impl From<FeedChangedEvent> for EntryProjectionInput {
    fn from(event: FeedChangedEvent) -> Self {
        Self::builder()
            .feed_url(event.feed_url)
            .crawl_job_id(event.crawl_job_id)
            .build()
    }
}

impl TryFrom<Event> for EntryProjectionInput {
    type Error = ProcessorError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Feed(FeedEvent::Discovered(event)) => Ok(event.into()),
            Event::Feed(FeedEvent::Changed(event)) => Ok(event.into()),
            event => Err(ProcessorError::UnexpectedEvent {
                expected: "entry projection event",
                actual: event.kind(),
            }),
        }
    }
}

/// Projects accepted feed sources into registry entry state.
#[derive(Debug, Clone)]
pub struct EntryProj;

impl EntryProj {
    /// Creates an entry projection processor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for EntryProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for EntryProj {
    type Input = EntryProjectionInput;
    type Phase = Transactional;

    fn id(&self) -> ProcessorId {
        ProcessorId::EntryProjection
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([
            FeedEventKind::Discovered.into(),
            FeedEventKind::Changed.into(),
        ])
    }
}

impl<S> Consumer<S> for EntryProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobStoreTx + EntryProjectionTx + JournalTx + Send,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<()> {
        let mut pj = cx.entry_projection();
        let source = pj.load_source(&input).await?;
        let body = pj.load_body(&source).await?;
        let feed = FeedService::parse_feed(source.feed_url.clone(), body.as_slice())?;
        let appearances = EntryAppearances::from_feed(&feed);
        let existing = pj.load_entries(&source, &appearances).await?;
        let changes = EntryReconciliation::new(source, appearances, existing).reconcile();
        pj.apply_entry_changes(changes).await?;
        Ok(())
    }
}

/// Transaction-scoped operations for projecting feed entries.
pub struct EntryProjectionScope<'a, Tx> {
    tx: &'a mut Tx,
    recorded: &'a mut RecordedEvents,
}

impl<'a, Tx> EntryProjectionScope<'a, Tx> {
    /// Creates a projection scope inside one open registry transaction.
    pub fn new(tx: &'a mut Tx, recorded: &'a mut RecordedEvents) -> Self {
        Self { tx, recorded }
    }
}

impl<Tx> EntryProjectionScope<'_, Tx>
where
    Tx: BlobStoreTx + EntryProjectionTx + JournalTx + Send,
{
    /// Returns the accepted feed source behind a feed lifecycle event.
    pub async fn load_source(
        &mut self,
        input: &EntryProjectionInput,
    ) -> RegistryDbResult<FeedSource> {
        let Some(source) = self.tx.load_entry_source(&input.crawl_job_id).await? else {
            return Err(RegistryDbError::internal(anyhow!(
                "entry projection source not found for crawl job {}",
                input.crawl_job_id
            )));
        };
        if source.feed_url != input.feed_url {
            return Err(RegistryDbError::internal(anyhow!(
                "entry projection source feed URL mismatch: event={}, source={}",
                input.feed_url,
                source.feed_url
            )));
        }
        Ok(source)
    }

    /// Returns the fetched response body for the given feed source.
    pub async fn load_body(&mut self, source: &FeedSource) -> RegistryDbResult<Vec<u8>> {
        self.tx.load_blob(source.body_blob).await
    }

    /// Returns existing registry entries addressed by the given appearances.
    pub async fn load_entries(
        &mut self,
        source: &FeedSource,
        appearances: &EntryAppearances,
    ) -> RegistryDbResult<EntrySet> {
        let entry_ids = appearances.ids();
        self.tx.load_entries(&source.feed_url, &entry_ids).await
    }

    /// Applies reconciled entry changes and records entry lifecycle events.
    pub async fn apply_entry_changes(&mut self, changes: EntryChanges) -> RegistryDbResult<()> {
        let events = entry_events(&changes);
        self.tx.apply_entry_changes(changes).await?;
        for event in events {
            self.record_event(event).await?;
        }
        Ok(())
    }

    async fn record_event<E>(&mut self, event: E) -> RegistryDbResult<()>
    where
        E: Into<Event>,
    {
        let event = event.into();
        let kind = event.kind();
        self.tx.append_event(event).await?;
        self.recorded.push(kind);
        Ok(())
    }
}

fn entry_events(changes: &EntryChanges) -> Vec<Event> {
    changes
        .iter()
        .filter_map(|change| match change {
            EntryChange::Discovered(entry) => Some(
                EntryDiscoveredEvent::new(
                    entry.feed_url.clone(),
                    entry.id.clone(),
                    entry.source.crawl_job_id.clone(),
                )
                .into(),
            ),
            EntryChange::Changed(entry) => Some(
                EntryChangedEvent::new(
                    entry.feed_url.clone(),
                    entry.id.clone(),
                    entry.source.crawl_job_id.clone(),
                )
                .into(),
            ),
            EntryChange::AlreadySeen(_) => None,
        })
        .collect()
}
