use bon::Builder;
use chrono::{DateTime, Utc};
use synd_feed::feed::service::FeedService;
use synd_feed::types::FeedUrl;
use tracing::debug;

use crate::{
    crawl::{blob::BlobRef, job::CrawlJobId},
    db::{BlobDb, EntryStore, FeedRegistryDb},
    entry::{EntryAppearances, EntryChange, EntryChanges, EntryReconciliation},
    event::{
        EntryChangedEvent, EntryDiscoveredEvent, Event, EventInput, EventType, FeedChangedEvent,
        FeedDiscoveredEvent, Processor, ProcessorError, ProcessorId, ProcessorResult, Projector,
        RegistryEvent,
    },
    feed::FeedSource,
};

/// Event input used to project entry state.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct EntryProjInput {
    pub feed_url: FeedUrl,
    pub crawl_job_id: CrawlJobId,
    pub body_blob: BlobRef,
    pub occurred_at: DateTime<Utc>,
}

impl EntryProjInput {
    fn from_feed_event(
        feed_url: FeedUrl,
        crawl_job_id: CrawlJobId,
        body_blob: BlobRef,
        occurred_at: DateTime<Utc>,
    ) -> Self {
        Self::builder()
            .feed_url(feed_url)
            .crawl_job_id(crawl_job_id)
            .body_blob(body_blob)
            .occurred_at(occurred_at)
            .build()
    }

    fn into_source(self) -> FeedSource {
        FeedSource::builder()
            .feed_url(self.feed_url)
            .crawl_job_id(self.crawl_job_id)
            .body_blob(self.body_blob)
            .seen_at(self.occurred_at)
            .build()
    }
}

impl EventInput for EntryProjInput {
    const INTERESTS: &'static [EventType] = &[FeedDiscoveredEvent::TYPE, FeedChangedEvent::TYPE];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::FeedDiscovered(event) => Ok(Self::from_feed_event(
                event.feed_url,
                event.crawl_job_id,
                event.body_blob,
                occurred_at,
            )),
            Event::FeedChanged(event) => Ok(Self::from_feed_event(
                event.feed_url,
                event.crawl_job_id,
                event.body_blob,
                occurred_at,
            )),
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
    type Input = EntryProjInput;

    fn id(&self) -> ProcessorId {
        ProcessorId::EntryProjection
    }
}

impl<S> Projector<S> for EntryProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobDb + EntryStore + Send,
{
    async fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let feed_url = input.feed_url.clone();
        let crawl_job_id = input.crawl_job_id.clone();
        let source = input.into_source();
        let body = tx.load_blob(source.body_blob).await?;
        let feed = FeedService::parse_feed(source.feed_url.clone(), body.as_slice())?;
        let appearances = EntryAppearances::from_feed(&feed);
        let entry_ids = appearances.ids();
        let existing = tx.load_entries(&source.feed_url, &entry_ids).await?;
        let changes = EntryReconciliation::new(source, appearances, existing).reconcile();
        let events = changes.lifecycle_events();
        let counts = EntryProjectionCounts::from_changes(&changes);
        debug!(
            feed_url = feed_url.as_str(),
            job_id = %crawl_job_id,
            discovered = counts.discovered,
            changed = counts.changed,
            "entry state projected"
        );
        tx.apply_entry_changes(changes).await?;
        Ok(events)
    }
}

/// Counts of entry changes produced by one projection batch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EntryProjectionCounts {
    discovered: usize,
    changed: usize,
}

impl EntryProjectionCounts {
    fn from_changes(changes: &EntryChanges) -> Self {
        let mut counts = Self {
            discovered: 0,
            changed: 0,
        };
        for change in changes.iter() {
            match change {
                EntryChange::Discovered(_) => counts.discovered += 1,
                EntryChange::Changed(_) => counts.changed += 1,
            }
        }
        counts
    }
}

impl EntryChanges {
    /// The lifecycle events represented by these changes.
    fn lifecycle_events(&self) -> Vec<Event> {
        self.iter()
            .map(|change| match change {
                EntryChange::Discovered(entry) => {
                    EntryDiscoveredEvent::new(entry.feed_url.clone(), entry.id.clone()).into()
                }
                EntryChange::Changed(entry) => {
                    EntryChangedEvent::new(entry.feed_url.clone(), entry.id.clone()).into()
                }
            })
            .collect()
    }
}
