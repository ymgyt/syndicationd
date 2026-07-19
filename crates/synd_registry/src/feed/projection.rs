use synd_feed::feed::service::FeedService;

use chrono::{DateTime, Utc};
use tracing::debug;

use crate::{
    db::{BlobStore, FeedRegistryDb, FeedStore},
    event::{
        CrawlJobFinishedEvent, Event, EventInput, EventType, Processor, ProcessorId,
        ProcessorResult, Projector, RegistryEvent,
    },
    feed::{FeedSource, UpsertFeedCommand},
};

/// Event input used to project feed state: a finished crawl and when it was
/// recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedProjInput {
    pub event: CrawlJobFinishedEvent,
    pub occurred_at: DateTime<Utc>,
}

impl EventInput for FeedProjInput {
    const INTERESTS: &'static [EventType] = &[CrawlJobFinishedEvent::TYPE];

    fn from_event(event: Event, occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::CrawlJobFinished(event) => Ok(Self { event, occurred_at }),
            event => Err(crate::event::ProcessorError::unexpected_input(
                "crawl job finished event",
                &event,
            )),
        }
    }
}

impl FeedProjInput {
    /// The feed source observed by the finished crawl, if it accepted a body.
    fn into_source(self) -> Option<FeedSource> {
        let body_blob = self.event.body_blob?;
        Some(
            FeedSource::builder()
                .feed_url(self.event.feed_url)
                .crawl_job_id(self.event.job_id)
                .body_blob(body_blob)
                .seen_at(self.occurred_at)
                .build(),
        )
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
    for<'tx> S::Tx<'tx>: BlobStore + FeedStore + Send,
{
    async fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let Some(source) = input.into_source() else {
            // Crawls without an accepted body (failures, not-modified) leave
            // no source; nothing to project.
            return Ok(Vec::new());
        };
        let body = tx.load_blob(source.body_blob).await?;
        let feed = FeedService::parse_feed(source.feed_url.clone(), body.as_slice())?;

        let command = UpsertFeedCommand::builder()
            .source(source)
            .meta(feed.meta().clone())
            .build();
        let source = command.source.clone();
        let outcome = tx.upsert_feed(command).await?;
        debug!(
            feed_url = source.feed_url.as_str(),
            job_id = %source.crawl_job_id,
            outcome = outcome.as_str(),
            "feed state projected"
        );
        Ok(outcome.into_event(&source).into_iter().collect())
    }
}
