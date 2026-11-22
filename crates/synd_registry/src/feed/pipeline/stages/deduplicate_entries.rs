use std::collections::{HashMap, hash_map::Entry as MapEntry};
use synd_feed::entry::Entry;

use crate::{
    db::FeedDb,
    event::ProcessorResult,
    feed::pipeline::{Ctx, EntryBatch, Stage},
};

pub(in crate::feed::pipeline) struct DeduplicateEntries;

impl Stage for DeduplicateEntries {
    type Input = EntryBatch<Entry>;
    type Output = EntryBatch<Entry>;

    async fn run<Tx: FeedDb + Send>(
        &self,
        mut batch: Self::Input,
        ctx: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Self::Output> {
        let mut first_index = HashMap::new();
        let mut unique = Vec::with_capacity(batch.entries.len());
        for entry in batch.entries {
            match first_index.entry(entry.id().clone()) {
                MapEntry::Vacant(slot) => {
                    slot.insert(unique.len());
                    unique.push(entry);
                }
                MapEntry::Occupied(slot) if unique[*slot.get()] != entry => {
                    ctx.skip_conflicting_entry(entry.id(), &batch.source);
                }
                MapEntry::Occupied(_) => {}
            }
        }
        unique.retain(|entry| !ctx.conflicting_ids.contains(entry.id()));
        batch.entries = unique;
        Ok(batch)
    }
}
