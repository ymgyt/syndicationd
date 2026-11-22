use synd_feed::{entry::SourceEntry, feed::service::FeedService};

use crate::{
    db::FeedDb,
    event::ProcessorResult,
    feed::pipeline::{Ctx, EntryBatch, FeedBody, Stage},
};

pub(in crate::feed::pipeline) struct ParseFeed;

impl Stage for ParseFeed {
    type Input = FeedBody;
    type Output = EntryBatch<SourceEntry>;

    async fn run<Tx: FeedDb + Send>(
        &self,
        body: FeedBody,
        _: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Self::Output> {
        let (meta, entries) =
            FeedService::parse_source(body.source.feed_url.clone(), body.bytes.as_slice())?;
        Ok(EntryBatch {
            source: body.source,
            meta,
            entries,
        })
    }
}
