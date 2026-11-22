use chrono::{DateTime, Utc};

use crate::{
    db::{BlobDb, FeedDb, FeedRegistryDb},
    event::{
        CrawlJobFinishedEvent, Event, EventInput, EventType, Processor, ProcessorId,
        ProcessorResult, Projector, RegistryEvent,
    },
    feed::FeedSource,
};

use super::pipeline::{Ctx, FeedBody, Stage, default_pipeline};

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
        let bytes = tx.load_blob(source.body_blob).await?;
        default_pipeline()
            .run(FeedBody { source, bytes }, &mut Ctx::new(tx))
            .await
    }
}
