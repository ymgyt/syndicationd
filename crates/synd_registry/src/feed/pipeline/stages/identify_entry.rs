use synd_feed::{
    entry::{IdentifiedEntry, SourceEntry},
    types::FeedMeta,
};

use crate::{
    db::FeedDb,
    event::ProcessorResult,
    feed::pipeline::{Ctx, EntryStage},
};

pub(in crate::feed::pipeline) struct IdentifyEntry;

impl EntryStage for IdentifyEntry {
    type Input = SourceEntry;
    type Output = IdentifiedEntry;

    async fn run_entry<Tx: FeedDb + Send>(
        &self,
        input: SourceEntry,
        meta: &FeedMeta,
        _: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Option<IdentifiedEntry>> {
        match IdentifiedEntry::identify(meta.url(), meta.r#type(), input) {
            Ok(entry) => Ok(Some(entry)),
            Err(error) => {
                tracing::warn!(%error, feed_url = %meta.url(), "skip feed entry because EntryId cannot be generated");
                Ok(None)
            }
        }
    }
}
