//! Typed stages for projecting one accepted feed body.
mod stages;

use crate::{
    db::FeedDb,
    event::{Event, ProcessorResult},
    feed::FeedSource,
};
use stages::{
    ApplyFeedUpdate, BuildFeedUpdate, ConvertSourceFields, DeduplicateEntries,
    DetermineEntryChange, IdentifyEntry, LoadStoredEntries, ParseFeed, SourceCategories,
};
use std::collections::HashSet;
use synd_feed::{entry::EntryId, types::FeedMeta};

pub(super) struct FeedBody {
    pub source: FeedSource,
    pub bytes: Vec<u8>,
}

/// Per-run dependencies and decisions shared across stages.
pub(super) struct Ctx<'a, Tx> {
    tx: &'a mut Tx,
    conflicting_ids: HashSet<EntryId>,
}

impl<'a, Tx> Ctx<'a, Tx> {
    pub fn new(tx: &'a mut Tx) -> Self {
        Self {
            tx,
            conflicting_ids: HashSet::new(),
        }
    }

    fn skip_conflicting_entry(&mut self, id: &EntryId, source: &FeedSource) {
        if self.conflicting_ids.insert(id.clone()) {
            tracing::warn!(entry_id = %id, feed_url = %source.feed_url, "skipping conflicting declarations of one entry");
        }
    }
}

pub(super) trait Stage: Sync {
    type Input: Send;
    type Output: Send;

    fn run<Tx: FeedDb + Send>(
        &self,
        input: Self::Input,
        ctx: &mut Ctx<'_, Tx>,
    ) -> impl Future<Output = ProcessorResult<Self::Output>> + Send;

    fn then<Next: Stage<Input = Self::Output>>(self, next: Next) -> Then<Self, Next>
    where
        Self: Sized,
    {
        Then(self, next)
    }
}

pub(super) struct Then<A, B>(A, B);

impl<A: Stage, B: Stage<Input = A::Output>> Stage for Then<A, B> {
    type Input = A::Input;
    type Output = B::Output;

    async fn run<Tx: FeedDb + Send>(
        &self,
        input: A::Input,
        ctx: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<B::Output> {
        let output = self.0.run(input, ctx).await?;
        self.1.run(output, ctx).await
    }
}

struct EntryBatch<T> {
    source: FeedSource,
    meta: FeedMeta,
    entries: Vec<T>,
}

/// A per-entry stage may skip an individual entry or fail the feed.
trait EntryStage: Sync {
    type Input: Send;
    type Output: Send;
    fn run_entry<Tx: FeedDb + Send>(
        &self,
        input: Self::Input,
        meta: &FeedMeta,
        ctx: &mut Ctx<'_, Tx>,
    ) -> impl Future<Output = ProcessorResult<Option<Self::Output>>> + Send;
}

struct ForEachEntry<S>(S);

impl<S: EntryStage> Stage for ForEachEntry<S> {
    type Input = EntryBatch<S::Input>;
    type Output = EntryBatch<S::Output>;

    async fn run<Tx: FeedDb + Send>(
        &self,
        batch: Self::Input,
        ctx: &mut Ctx<'_, Tx>,
    ) -> ProcessorResult<Self::Output> {
        let mut entries = Vec::with_capacity(batch.entries.len());
        for input in batch.entries {
            if let Some(entry) = self.0.run_entry(input, &batch.meta, ctx).await? {
                entries.push(entry);
            }
        }
        Ok(EntryBatch {
            meta: batch.meta,
            source: batch.source,
            entries,
        })
    }
}

struct EntryPipeline;

impl EntryPipeline {
    fn new() -> Self {
        Self
    }
}

// The empty pipeline accepts a loaded feed body. Appending stages refines its output type.
impl Stage for EntryPipeline {
    type Input = FeedBody;
    type Output = FeedBody;

    fn run<Tx: FeedDb + Send>(
        &self,
        body: FeedBody,
        _: &mut Ctx<'_, Tx>,
    ) -> impl Future<Output = ProcessorResult<FeedBody>> + Send {
        std::future::ready(Ok(body))
    }
}

pub(super) fn default_pipeline() -> impl Stage<Input = FeedBody, Output = Vec<Event>> {
    EntryPipeline::new()
        .then(ParseFeed)
        .then(ForEachEntry(IdentifyEntry))
        .then(ForEachEntry(ConvertSourceFields))
        .then(SourceCategories { rss_delimiter: ',' })
        .then(DeduplicateEntries)
        .then(LoadStoredEntries)
        .then(ForEachEntry(DetermineEntryChange))
        .then(BuildFeedUpdate)
        .then(ApplyFeedUpdate)
}
