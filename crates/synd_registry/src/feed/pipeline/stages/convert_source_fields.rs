use synd_feed::{
    entry::{Entry, IdentifiedEntry},
    types::FeedMeta,
};

use crate::{
    db::FeedDb,
    event::ProcessorResult,
    feed::pipeline::{Ctx, EntryStage},
};

pub(in crate::feed::pipeline) struct ConvertSourceFields;

impl EntryStage for ConvertSourceFields {
    type Input = IdentifiedEntry;
    type Output = Entry;

    async fn run_entry<Tx: FeedDb + Send>(
        &self,
        input: IdentifiedEntry,
        _: &FeedMeta,
        _: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Option<Entry>> {
        Ok(Some(input.into_entry()))
    }
}
