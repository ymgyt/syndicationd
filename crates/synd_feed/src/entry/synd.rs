use bon::Builder;
use thiserror::Error;

use crate::types::Time;

use super::{Entry, EntryId};

/// Canonical ordering context assigned when an entry is first observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryOrderKey(Time);

impl EntryOrderKey {
    /// Reconstructs a persisted order key.
    pub fn from_datetime(time: Time) -> Self {
        Self(time)
    }

    /// Returns the timestamp used for canonical entry ordering.
    pub fn as_datetime(self) -> Time {
        self.0
    }
}

impl Entry {
    /// Resolves the canonical order key for this entry's first observation.
    pub fn resolve_order_key(&self, observed_at: Time) -> EntryOrderKey {
        self.published().or(self.updated()).map_or_else(
            || EntryOrderKey::from_datetime(observed_at),
            EntryOrderKey::from_datetime,
        )
    }
}

/// A feed-declared entry composed with context owned by synd.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct SyndEntry {
    entry: Entry,
    order_key: EntryOrderKey,
}

impl SyndEntry {
    /// Returns the value declared by the feed.
    pub fn entry(&self) -> &Entry {
        &self.entry
    }

    /// Returns the immutable canonical order key.
    pub fn order_key(&self) -> EntryOrderKey {
        self.order_key
    }

    /// Compares the stored entry with a new observation of the same identity.
    pub fn compute_diff(&self, observed: &Entry) -> Result<SyndEntryDiff, EntryIdMismatch> {
        if self.entry.id() != observed.id() {
            return Err(EntryIdMismatch {
                current: self.entry.id().clone(),
                observed: observed.id().clone(),
            });
        }

        Ok(if &self.entry == observed {
            SyndEntryDiff::Unchanged
        } else {
            SyndEntryDiff::EntryChanged
        })
    }
}

/// Difference between a stored syndicated entry and its latest observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyndEntryDiff {
    Unchanged,
    EntryChanged,
}

/// Error returned when entries with different identities are compared.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("cannot compare entries with different ids: current={current}, observed={observed}")]
pub struct EntryIdMismatch {
    current: EntryId,
    observed: EntryId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_key_prefers_published_time() {
        let published = time("2026-07-18T12:00:00Z");
        let updated = time("2026-07-19T12:00:00Z");
        let observed_at = time("2026-07-20T12:00:00Z");
        let entry = Entry::builder()
            .id(entry_id('a'))
            .updated(updated)
            .published(published)
            .build();

        assert_eq!(
            entry.resolve_order_key(observed_at).as_datetime(),
            published
        );
    }

    #[test]
    fn order_key_falls_back_to_updated_time() {
        let updated = time("2026-07-19T12:00:00Z");
        let observed_at = time("2026-07-20T12:00:00Z");
        let entry = Entry::builder().id(entry_id('a')).updated(updated).build();

        assert_eq!(entry.resolve_order_key(observed_at).as_datetime(), updated);
    }

    #[test]
    fn order_key_falls_back_to_observed_time() {
        let observed_at = time("2026-07-20T12:00:00Z");
        let entry = Entry::builder().id(entry_id('a')).build();

        assert_eq!(
            entry.resolve_order_key(observed_at).as_datetime(),
            observed_at
        );
    }

    #[test]
    fn diff_is_unchanged_for_the_same_entry() {
        let current = Entry::builder().id(entry_id('a')).build();
        let synd_entry = SyndEntry::builder()
            .entry(current.clone())
            .order_key(current.resolve_order_key(time("2026-07-20T12:00:00Z")))
            .build();

        assert_eq!(
            synd_entry.compute_diff(&current),
            Ok(SyndEntryDiff::Unchanged)
        );
    }

    #[test]
    fn diff_detects_a_changed_entry_value() {
        let current = Entry::builder().id(entry_id('a')).build();
        let synd_entry = SyndEntry::builder()
            .entry(current)
            .order_key(EntryOrderKey::from_datetime(time("2026-07-20T12:00:00Z")))
            .build();
        let observed = Entry::builder()
            .id(entry_id('a'))
            .updated(time("2026-07-21T12:00:00Z"))
            .build();

        assert_eq!(
            synd_entry.compute_diff(&observed),
            Ok(SyndEntryDiff::EntryChanged)
        );
    }

    #[test]
    fn synd_entry_diff_rejects_different_entry_ids() {
        let current_id = entry_id('a');
        let observed_id = entry_id('b');
        let current = Entry::builder().id(current_id.clone()).build();
        let observed = Entry::builder().id(observed_id.clone()).build();
        let synd_entry = SyndEntry::builder()
            .entry(current)
            .order_key(EntryOrderKey::from_datetime(time("2026-07-20T12:00:00Z")))
            .build();

        assert_eq!(
            synd_entry.compute_diff(&observed),
            Err(EntryIdMismatch {
                current: current_id,
                observed: observed_id,
            })
        );
    }

    fn entry_id(digit: char) -> EntryId {
        EntryId::parse(format!("synd:entry:v1:{}", digit.to_string().repeat(64))).unwrap()
    }

    fn time(value: &str) -> Time {
        value.parse().unwrap()
    }
}
