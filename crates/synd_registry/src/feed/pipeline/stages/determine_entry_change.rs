use synd_feed::types::FeedMeta;

use super::EntryWithPrevious;
use crate::{
    db::FeedDb,
    entry::EntryChange,
    event::ProcessorResult,
    feed::FeedUpdateError,
    feed::pipeline::{Ctx, EntryStage},
};

pub(in crate::feed::pipeline) struct DetermineEntryChange;

impl EntryStage for DetermineEntryChange {
    type Input = EntryWithPrevious;
    type Output = EntryChange;

    async fn run_entry<Tx: FeedDb + Send>(
        &self,
        input: EntryWithPrevious,
        _: &FeedMeta,
        _: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Option<EntryChange>> {
        let change = EntryChange::determine(input.previous, input.observed)
            .map_err(FeedUpdateError::from)?;
        Ok(Some(change))
    }
}
