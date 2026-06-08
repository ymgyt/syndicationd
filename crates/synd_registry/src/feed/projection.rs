use synd_feed::feed::service::FeedService;

use crate::{
    db::{BlobStoreTx, FeedProjectionTx, FeedRegistryDb},
    event::{
        ConsumeContext, Consumer, CrawlEvent, CrawlEventKind, CrawlJobFinishedEvent, Event,
        EventInterests, JournalTx, Processor, ProcessorError, ProcessorId, ProcessorResult,
        Transactional,
    },
    feed::UpsertFeedCommand,
};

/// Event input used to project feed state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedProjectionInput {
    event: CrawlJobFinishedEvent,
}

impl FeedProjectionInput {
    /// Creates feed projection input from one finished crawl job.
    pub fn new(event: CrawlJobFinishedEvent) -> Self {
        Self { event }
    }
}

impl TryFrom<Event> for FeedProjectionInput {
    type Error = ProcessorError;

    fn try_from(event: Event) -> Result<Self, Self::Error> {
        match event {
            Event::Crawl(CrawlEvent::JobFinished(event)) => Ok(Self::new(event)),
            event => Err(ProcessorError::UnexpectedEvent {
                expected: "feed projection event",
                actual: event.kind(),
            }),
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
    type Input = FeedProjectionInput;
    type Phase = Transactional;

    fn id(&self) -> ProcessorId {
        ProcessorId::FeedProjection
    }

    fn interests(&self) -> EventInterests {
        EventInterests::new([CrawlEventKind::JobFinished.into()])
    }
}

impl<S> Consumer<S> for FeedProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobStoreTx + FeedProjectionTx + JournalTx + Send,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<()> {
        let mut pj = cx.feed_projection();
        let Some(source) = pj.load_feed_source(&input.event.job_id).await? else {
            return Ok(());
        };
        let body = pj.load_body(&source).await?;
        let feed = FeedService::parse_feed(source.feed_url.clone(), body.as_slice())?;

        pj.upsert_feed(
            UpsertFeedCommand::builder()
                .source(source)
                .meta(feed.meta().clone())
                .build(),
        )
        .await?;
        Ok(())
    }
}
