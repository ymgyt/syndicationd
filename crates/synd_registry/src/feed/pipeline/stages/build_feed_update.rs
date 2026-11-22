use crate::{
    db::FeedDb,
    entry::EntryChange,
    event::ProcessorResult,
    feed::FeedUpdate,
    feed::pipeline::{Ctx, EntryBatch, Stage},
};

pub(in crate::feed::pipeline) struct BuildFeedUpdate;

impl Stage for BuildFeedUpdate {
    type Input = EntryBatch<EntryChange>;
    type Output = FeedUpdate;

    async fn run<Tx: FeedDb + Send>(
        &self,
        batch: Self::Input,
        ctx: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<FeedUpdate> {
        let membership = batch
            .entries
            .iter()
            .map(|change| change.entry_id().clone())
            .collect();
        let changes = batch
            .entries
            .into_iter()
            .filter_map(|change| change.into_write(batch.source.seen_at))
            .collect();
        Ok(FeedUpdate {
            source: batch.source,
            meta: batch.meta,
            membership,
            preserve_membership: std::mem::take(&mut ctx.conflicting_ids),
            entry_changes: changes,
        })
    }
}
