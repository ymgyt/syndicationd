use synd_feed::feed::service::FeedService;

use crate::{
    db::{BlobStoreTx, FeedProjectionTx, FeedRegistryDb},
    event::{
        ConsumeContext, Consumer, CrawlJobFinishedEvent, Event, Processor, ProcessorId,
        ProcessorResult,
    },
    feed::UpsertFeedCommand,
};

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

impl<S> Consumer<S> for FeedProj
where
    S: FeedRegistryDb,
    for<'tx> S::Tx<'tx>: BlobStoreTx + FeedProjectionTx + Send,
{
    async fn consume(
        &mut self,
        cx: &mut ConsumeContext<'_, S::Tx<'_>>,
        input: Self::Input,
    ) -> ProcessorResult<Vec<Event>> {
        let mut pj = cx.feed_projection();
        let Some(source) = pj.load_feed_source(&input.job_id).await? else {
            return Ok(Vec::new());
        };
        let body = pj.load_body(&source).await?;
        let feed = FeedService::parse_feed(source.feed_url.clone(), body.as_slice())?;

        let (_, events) = pj
            .upsert_feed(
                UpsertFeedCommand::builder()
                    .source(source)
                    .meta(feed.meta().clone())
                    .build(),
            )
            .await?;
        Ok(events)
    }
}
