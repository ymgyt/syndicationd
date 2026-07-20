use std::collections::{BTreeMap, btree_map::Entry as BTreeMapEntry};

use synd_feed::{
    entry::{Entry, EntryId, EntryIdMismatch},
    types::{Feed, FeedMeta, FeedUrl, Time},
};
use thiserror::Error;

use crate::entry::{Change, Changes, Entries};

use super::FeedSource;

/// One accepted feed body parsed and normalized for reconciliation.
#[derive(Debug, Clone)]
pub(super) struct FeedObservation {
    source: FeedSource,
    meta: FeedMeta,
    entries: ObservedEntries,
}

impl FeedObservation {
    pub(super) fn from_feed(source: FeedSource, feed: Feed) -> Result<Self, FeedUpdateError> {
        let (meta, entries) = feed.parts();
        let entries = ObservedEntries::collect(&source.feed_url, entries)?;
        Ok(Self {
            source,
            meta,
            entries,
        })
    }

    pub(super) fn membership(&self) -> &[EntryId] {
        &self.entries.membership
    }

    pub(super) fn decide(self, current: Entries) -> Result<FeedUpdate, FeedUpdateError> {
        FeedUpdate::decide(self, current)
    }
}

/// Entry declarations from one feed body with conflicting duplicate IDs rejected.
#[derive(Debug, Clone)]
struct ObservedEntries {
    membership: Vec<EntryId>,
    entries: Vec<Entry>,
}

impl ObservedEntries {
    fn collect(feed_url: &FeedUrl, entries: Vec<Entry>) -> Result<Self, FeedUpdateError> {
        let mut unique = BTreeMap::new();
        for entry in entries {
            let entry_id = entry.id().clone();
            match unique.entry(entry_id.clone()) {
                BTreeMapEntry::Vacant(slot) => {
                    slot.insert(entry);
                }
                BTreeMapEntry::Occupied(slot) if slot.get() == &entry => {}
                BTreeMapEntry::Occupied(_) => {
                    return Err(FeedUpdateError::ConflictingEntry {
                        feed_url: feed_url.clone(),
                        entry_id,
                    });
                }
            }
        }

        Ok(Self {
            membership: unique.keys().cloned().collect(),
            entries: unique.into_values().collect(),
        })
    }

    fn reconcile(
        self,
        mut current: Entries,
        observed_at: Time,
    ) -> Result<ReconciledEntries, EntryIdMismatch> {
        let changes = self
            .entries
            .into_iter()
            .map(|observed| {
                let current = current.remove(observed.id());
                Change::decide(current, observed, observed_at)
            })
            .filter_map(Result::transpose)
            .collect::<Result<Changes, _>>()?;

        Ok(ReconciledEntries {
            membership: self.membership,
            changes,
        })
    }
}

/// Current membership and entry transitions decided from one observation.
struct ReconciledEntries {
    membership: Vec<EntryId>,
    changes: Changes,
}

/// Atomic state transition derived from one accepted feed observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedUpdate {
    source: FeedSource,
    meta: FeedMeta,
    membership: Vec<EntryId>,
    entry_changes: Changes,
}

impl FeedUpdate {
    fn decide(observation: FeedObservation, current: Entries) -> Result<Self, FeedUpdateError> {
        let FeedObservation {
            source,
            meta,
            entries,
        } = observation;
        let reconciled = entries.reconcile(current, source.seen_at)?;

        Ok(Self {
            source,
            meta,
            membership: reconciled.membership,
            entry_changes: reconciled.changes,
        })
    }

    pub fn source(&self) -> &FeedSource {
        &self.source
    }

    pub fn meta(&self) -> &FeedMeta {
        &self.meta
    }

    pub fn membership(&self) -> &[EntryId] {
        &self.membership
    }

    pub fn entry_changes(&self) -> &Changes {
        &self.entry_changes
    }
}

/// Deterministic failure while deciding an accepted feed observation.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum FeedUpdateError {
    #[error(transparent)]
    EntryIdMismatch(#[from] EntryIdMismatch),
    #[error("feed {feed_url} contains conflicting entries with id {entry_id}")]
    ConflictingEntry {
        feed_url: FeedUrl,
        entry_id: EntryId,
    },
}

#[cfg(test)]
mod tests {
    use synd_feed::types::{FeedType, Time};

    use crate::crawl::{blob::BlobRef, job::CrawlJobId};

    use super::*;

    #[test]
    fn identical_duplicate_entries_are_collapsed() {
        let entry_id = entry_id('a');
        let entry = Entry::builder().id(entry_id.clone()).build();
        let observation =
            FeedObservation::from_feed(source(), feed(vec![entry.clone(), entry])).unwrap();
        let update = observation.decide(Entries::default()).unwrap();

        assert_eq!(update.membership(), &[entry_id]);
        let mut changes = update.entry_changes().into_iter();
        assert!(matches!(changes.next(), Some(Change::Discovered(_))));
        assert_eq!(changes.next(), None);
    }

    #[test]
    fn conflicting_duplicate_entries_are_rejected() {
        let feed_url = feed_url();
        let entry_id = entry_id('a');
        let first = Entry::builder().id(entry_id.clone()).build();
        let conflicting = Entry::builder()
            .id(entry_id.clone())
            .updated(time("2026-07-21T12:00:00Z"))
            .build();

        let error = FeedObservation::from_feed(
            source_with_url(feed_url.clone()),
            feed_with_url(feed_url.clone(), vec![first, conflicting]),
        )
        .unwrap_err();

        assert_eq!(
            error,
            FeedUpdateError::ConflictingEntry { feed_url, entry_id }
        );
    }

    fn source() -> FeedSource {
        source_with_url(feed_url())
    }

    fn source_with_url(feed_url: FeedUrl) -> FeedSource {
        FeedSource::builder()
            .feed_url(feed_url)
            .crawl_job_id(CrawlJobId::new("crawl-job"))
            .body_blob(BlobRef::new(1))
            .seen_at(time("2026-07-20T12:00:00Z"))
            .build()
    }

    fn feed(entries: Vec<Entry>) -> Feed {
        feed_with_url(feed_url(), entries)
    }

    fn feed_with_url(feed_url: FeedUrl, entries: Vec<Entry>) -> Feed {
        let meta = FeedMeta::builder()
            .url(feed_url)
            .feed_type(FeedType::Atom)
            .build();
        Feed::new(meta, entries)
    }

    fn feed_url() -> FeedUrl {
        FeedUrl::parse("https://example.com/feed.xml").unwrap()
    }

    fn entry_id(digit: char) -> EntryId {
        EntryId::parse(format!("synd:entry:v1:{}", digit.to_string().repeat(64))).unwrap()
    }

    fn time(value: &str) -> Time {
        value.parse().unwrap()
    }
}
