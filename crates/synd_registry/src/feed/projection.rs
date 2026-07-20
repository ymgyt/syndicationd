use synd_feed::feed::service::FeedService;
use synd_feed::types::FeedUrl;

use chrono::{DateTime, Utc};
use tracing::debug;

use crate::{
    crawl::job::CrawlJobId,
    db::{BlobDb, FeedDb, FeedRegistryDb},
    entry::{Change, Entries},
    event::{
        CrawlJobFinishedEvent, EntryChangedEvent, EntryDiscoveredEvent, Event, EventInput,
        EventType, Processor, ProcessorId, ProcessorResult, Projector, RegistryEvent,
    },
    feed::{FeedSource, FeedUpdate},
};

use super::update::FeedObservation;

/// Event input used to project feed state: a finished crawl and when it was
/// recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedProjInput {
    Accepted(FeedSource),
    NoAcceptedBody,
}

impl EventInput for FeedProjInput {
    const INTERESTS: &'static [EventType] = &[CrawlJobFinishedEvent::TYPE];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::CrawlJobFinished(event) => Ok(Self::new(event, occurred_at)),
            event => Err(crate::event::ProcessorError::unexpected_input(
                "crawl job finished event",
                &event,
            )),
        }
    }
}

impl FeedProjInput {
    pub fn new(event: CrawlJobFinishedEvent, occurred_at: DateTime<Utc>) -> Self {
        let Some(body_blob) = event.body_blob else {
            return Self::NoAcceptedBody;
        };
        Self::Accepted(
            FeedSource::builder()
                .feed_url(event.feed_url)
                .crawl_job_id(event.job_id)
                .body_blob(body_blob)
                .seen_at(occurred_at)
                .build(),
        )
    }
}

/// Accepted source body loaded from durable blob storage.
struct FeedBody {
    source: FeedSource,
    bytes: Vec<u8>,
}

impl FeedBody {
    async fn load<T>(tx: &mut T, source: FeedSource) -> ProcessorResult<Self>
    where
        T: BlobDb + Send,
    {
        let bytes = tx.load_blob(source.body_blob).await?;
        Ok(Self { source, bytes })
    }

    fn parse(self) -> ProcessorResult<FeedObservation> {
        let feed = FeedService::parse_feed(self.source.feed_url.clone(), self.bytes.as_slice())?;
        Ok(FeedObservation::from_feed(self.source, feed)?)
    }
}

/// Parsed observation paired with the current entries needed for a pure decision.
struct FeedUpdateInput {
    observation: FeedObservation,
    current: Entries,
}

impl FeedUpdateInput {
    async fn observe<T>(tx: &mut T, source: FeedSource) -> ProcessorResult<Self>
    where
        T: BlobDb + FeedDb + Send,
    {
        let observation = FeedBody::load(tx, source).await?.parse()?;
        let current = tx.load_entries(observation.membership()).await?;
        Ok(Self {
            observation,
            current,
        })
    }

    fn decide(self) -> ProcessorResult<FeedUpdate> {
        Ok(self.observation.decide(self.current)?)
    }
}

/// Events and diagnostics derived once from an update applied in the current transaction.
struct AppliedFeedUpdate {
    feed_url: FeedUrl,
    crawl_job_id: CrawlJobId,
    events: Vec<Event>,
    discovered: usize,
    changed: usize,
}

impl AppliedFeedUpdate {
    async fn apply<T>(tx: &mut T, update: &FeedUpdate) -> ProcessorResult<Self>
    where
        T: FeedDb + Send,
    {
        tx.apply_feed_update(update).await?;
        Ok(Self::from(update))
    }

    fn record(&mut self, change: &Change) {
        let entry_id = change.entry().entry().id().clone();
        let event = match change {
            Change::Discovered(_) => {
                self.discovered += 1;
                EntryDiscoveredEvent::new(self.feed_url.clone(), entry_id).into()
            }
            Change::Changed(_) => {
                self.changed += 1;
                EntryChangedEvent::new(self.feed_url.clone(), entry_id).into()
            }
        };
        self.events.push(event);
    }

    fn log(&self) {
        debug!(
            feed_url = self.feed_url.as_str(),
            job_id = %self.crawl_job_id,
            discovered = self.discovered,
            changed = self.changed,
            "feed state projected"
        );
    }

    fn into_events(self) -> Vec<Event> {
        self.events
    }
}

impl From<&FeedUpdate> for AppliedFeedUpdate {
    fn from(update: &FeedUpdate) -> Self {
        let mut applied = Self {
            feed_url: update.source().feed_url.clone(),
            crawl_job_id: update.source().crawl_job_id.clone(),
            events: Vec::new(),
            discovered: 0,
            changed: 0,
        };
        for change in update.entry_changes() {
            applied.record(change);
        }
        applied
    }
}

/// Projects successful crawl results into feed state.
#[derive(Debug, Clone)]
pub struct FeedProj;

impl FeedProj {
    /// Creates a feed projection processor.
    pub fn new() -> Self {
        Self
    }
}

impl Default for FeedProj {
    fn default() -> Self {
        Self::new()
    }
}

impl Processor for FeedProj {
    type Input = FeedProjInput;

    fn id(&self) -> ProcessorId {
        ProcessorId::FeedProjection
    }
}

impl<S> Projector<S> for FeedProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobDb + FeedDb + Send,
{
    async fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let FeedProjInput::Accepted(source) = input else {
            return Ok(Vec::new());
        };
        let update = FeedUpdateInput::observe(tx, source).await?.decide()?;
        let applied = AppliedFeedUpdate::apply(tx, &update).await?;
        applied.log();
        Ok(applied.into_events())
    }
}
