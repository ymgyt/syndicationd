use bon::Builder;
use chrono::{DateTime, Utc};
use synd_feed::feed::service::FeedService;
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    crawl::job::CrawlJobId,
    db::{BlobStore, CrawlResultStore, EntryStore, FeedRegistryDb},
    entry::{EntryAppearances, EntryChange, EntryChanges, EntryReconciliation},
    error::{RegistryDbError, RegistryDbResult},
    event::{
        EntryChangedEvent, EntryDiscoveredEvent, Event, EventInput, EventType, FeedChangedEvent,
        FeedDiscoveredEvent, Processor, ProcessorError, ProcessorId, ProcessorResult, Projector,
        RegistryEvent,
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

impl EventInput for EntryProjectionInput {
    const INTERESTS: &'static [EventType] = &[FeedDiscoveredEvent::TYPE, FeedChangedEvent::TYPE];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::FeedDiscovered(event) => Ok(event.into()),
            Event::FeedChanged(event) => Ok(event.into()),
            event => Err(ProcessorError::unexpected_input(
                "entry projection event",
                &event,
            )),
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

    fn id(&self) -> ProcessorId {
        ProcessorId::EntryProjection
    }
}

impl<S> Projector<S> for EntryProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobStore + CrawlResultStore + EntryStore + Send,
{
    async fn apply(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let source = load_source(tx, &input).await?;
        let body = tx.load_blob(source.body_blob).await?;
        let feed = FeedService::parse_feed(source.feed_url.clone(), body.as_slice())?;
        let appearances = EntryAppearances::from_feed(&feed);
        let entry_ids = appearances.ids();
        let existing = tx.load_entries(&source.feed_url, &entry_ids).await?;
        let changes = EntryReconciliation::new(source, appearances, existing).reconcile();
        let events = entry_events(&changes);
        let counts = EntryProjectionCounts::from_changes(&changes);
        debug!(
            feed_url = input.feed_url.as_str(),
            job_id = %input.crawl_job_id,
            discovered = counts.discovered,
            changed = counts.changed,
            already_seen = counts.already_seen,
            "entry state projected"
        );
        tx.apply_entry_changes(changes).await?;
        Ok(events)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntryProjectionCounts {
    discovered: usize,
    changed: usize,
    already_seen: usize,
}

impl EntryProjectionCounts {
    fn from_changes(changes: &EntryChanges) -> Self {
        let mut counts = Self {
            discovered: 0,
            changed: 0,
            already_seen: 0,
        };
        for change in changes.iter() {
            match change {
                EntryChange::Discovered(_) => counts.discovered += 1,
                EntryChange::Changed(_) => counts.changed += 1,
                EntryChange::AlreadySeen(_) => counts.already_seen += 1,
            }
        }
        counts
    }
}

async fn load_source<Tx>(tx: &mut Tx, input: &EntryProjectionInput) -> RegistryDbResult<FeedSource>
where
    Tx: CrawlResultStore + Send,
{
    let Some(source) = tx.load_crawl_source(&input.crawl_job_id).await? else {
        return Err(RegistryDbError::internal_message(format!(
            "entry projection source not found for crawl job {}",
            input.crawl_job_id
        )));
    };
    if source.feed_url != input.feed_url {
        return Err(RegistryDbError::internal_message(format!(
            "entry projection source feed URL mismatch: event={}, source={}",
            input.feed_url, source.feed_url
        )));
    }
    Ok(source)
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
