use synd_feed::entry::Entry;

use super::EntryWithPrevious;
use crate::{
    db::FeedDb,
    event::ProcessorResult,
    feed::pipeline::{Ctx, EntryBatch, Stage},
};

pub(in crate::feed::pipeline) struct LoadStoredEntries;

impl Stage for LoadStoredEntries {
    type Input = EntryBatch<Entry>;
    type Output = EntryBatch<EntryWithPrevious>;

    async fn run<Tx: FeedDb + Send>(
        &self,
        batch: Self::Input,
        ctx: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Self::Output> {
        let ids: Vec<_> = batch
            .entries
            .iter()
            .map(|entry| entry.id().clone())
            .collect();
        let mut stored = ctx.tx.load_entries(&ids).await?;
        let entries = batch
            .entries
            .into_iter()
            .map(|observed| EntryWithPrevious {
                previous: stored.remove(observed.id()),
                observed,
            })
            .collect();
        Ok(EntryBatch {
            source: batch.source,
            meta: batch.meta,
            entries,
        })
    }
}
