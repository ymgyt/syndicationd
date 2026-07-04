use synd_feed::feed::service::FeedService;

use chrono::{DateTime, Utc};
use tracing::debug;

use crate::{
    db::{BlobStore, CrawlResultStore, FeedRegistryDb, FeedStore},
    event::{
        CrawlJobFinishedEvent, Event, EventInput, EventType, Processor, ProcessorId,
        ProcessorResult, Projector, RegistryEvent,
    },
    feed::UpsertFeedCommand,
};

impl EventInput for CrawlJobFinishedEvent {
    const INTERESTS: &'static [EventType] = &[Self::TYPE];

    fn from_event(event: Event, _occurred_at: DateTime<Utc>) -> ProcessorResult<Self> {
        match event {
            Event::CrawlJobFinished(event) => Ok(event),
            event => Err(crate::event::ProcessorError::unexpected_input(
                "crawl job finished event",
                &event,
            )),
        }
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
    type Input = CrawlJobFinishedEvent;

    fn id(&self) -> ProcessorId {
        ProcessorId::FeedProjection
    }
}

impl<S> Projector<S> for FeedProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobStore + CrawlResultStore + FeedStore + Send,
{
    async fn project(
        &mut self,
        tx: &mut S::Tx<'_>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let Some(source) = tx.load_crawl_source(&input.job_id).await? else {
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
            outcome = feed_projection_outcome(outcome),
            "feed state projected"
        );
        Ok(outcome.into_event(&source).into_iter().collect())
    }
}

fn feed_projection_outcome(outcome: crate::feed::UpsertFeedOutcome) -> &'static str {
    match outcome {
        crate::feed::UpsertFeedOutcome::Discovered => "discovered",
        crate::feed::UpsertFeedOutcome::Changed => "changed",
        crate::feed::UpsertFeedOutcome::Unchanged => "unchanged",
    }
}
