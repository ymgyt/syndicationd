use std::{collections::HashMap, iter::FromIterator, slice};

use synd_feed::{
    entry::{Entry, EntryId, EntryIdMismatch, SyndEntry, SyndEntryDiff},
    types::Time,
};

/// Existing syndicated entries indexed by their stable identity.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Entries {
    entries: HashMap<EntryId, SyndEntry>,
}

impl FromIterator<SyndEntry> for Entries {
    fn from_iter<T>(entries: T) -> Self
    where
        T: IntoIterator<Item = SyndEntry>,
    {
        let entries = entries
            .into_iter()
            .map(|entry| (entry.entry().id().clone(), entry))
            .collect();
        Self { entries }
    }
}

impl Entries {
    pub(crate) fn remove(&mut self, entry_id: &EntryId) -> Option<SyndEntry> {
        self.entries.remove(entry_id)
    }
}

/// A persisted entry transition decided from one feed observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Discovered(SyndEntry),
    Changed(SyndEntry),
}

impl Change {
    /// Returns the resulting entry state carried by this change.
    pub fn entry(&self) -> &SyndEntry {
        match self {
            Self::Discovered(entry) | Self::Changed(entry) => entry,
        }
    }
}

/// Result of comparing an observed source entry with its stored state.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EntryChange {
    Discovered(Entry),
    Unchanged(SyndEntry),
    Changed {
        previous: Box<SyndEntry>,
        observed: Entry,
    },
}

impl EntryChange {
    pub(crate) fn determine(
        current: Option<SyndEntry>,
        observed: Entry,
    ) -> Result<Self, EntryIdMismatch> {
        let Some(previous) = current else {
            return Ok(Self::Discovered(observed));
        };
        Ok(match previous.compute_diff(&observed)? {
            SyndEntryDiff::Unchanged => Self::Unchanged(previous),
            SyndEntryDiff::EntryChanged => Self::Changed {
                previous: Box::new(previous),
                observed,
            },
        })
    }

    pub(crate) fn entry_id(&self) -> &EntryId {
        match self {
            Self::Discovered(observed) | Self::Changed { observed, .. } => observed.id(),
            Self::Unchanged(current) => current.entry().id(),
        }
    }

    pub(crate) fn into_write(self, seen_at: Time) -> Option<Change> {
        match self {
            Self::Unchanged(_) => None,
            Self::Discovered(observed) => {
                let order_key = observed.resolve_order_key(seen_at);
                Some(Change::Discovered(
                    SyndEntry::builder()
                        .entry(observed)
                        .order_key(order_key)
                        .build(),
                ))
            }
            Self::Changed { previous, observed } => Some(Change::Changed(
                SyndEntry::builder()
                    .entry(observed)
                    .order_key(previous.order_key())
                    .build(),
            )),
        }
    }
}

/// Entry transitions produced by one accepted feed observation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Changes {
    changes: Vec<Change>,
}

impl FromIterator<Change> for Changes {
    fn from_iter<T>(changes: T) -> Self
    where
        T: IntoIterator<Item = Change>,
    {
        Self {
            changes: changes.into_iter().collect(),
        }
    }
}

impl Changes {
    pub fn iter(&self) -> slice::Iter<'_, Change> {
        self.changes.iter()
    }
}

impl<'a> IntoIterator for &'a Changes {
    type Item = &'a Change;
    type IntoIter = slice::Iter<'a, Change>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use synd_feed::entry::EntryOrderKey;

    use super::*;

    #[test]
    fn discovered_entry_resolves_its_order_key() {
        let published = time("2026-07-18T12:00:00Z");
        let observed_at = time("2026-07-20T12:00:00Z");
        let observed = Entry::builder()
            .id(entry_id('a'))
            .published(published)
            .build();

        let change = EntryChange::determine(None, observed.clone())
            .unwrap()
            .into_write(observed_at);

        let Some(Change::Discovered(entry)) = change else {
            panic!("expected a discovered entry");
        };
        assert_eq!(entry.entry(), &observed);
        assert_eq!(entry.order_key().as_datetime(), published);
    }

    #[test]
    fn changed_entry_preserves_its_current_order_key() {
        let order_key = time("2026-07-18T12:00:00Z");
        let current = SyndEntry::builder()
            .entry(Entry::builder().id(entry_id('a')).build())
            .order_key(EntryOrderKey::from_datetime(order_key))
            .build();
        let observed = Entry::builder()
            .id(entry_id('a'))
            .updated(time("2026-07-21T12:00:00Z"))
            .build();

        let change = EntryChange::determine(Some(current), observed.clone())
            .unwrap()
            .into_write(time("2026-07-21T12:00:00Z"));

        let Some(Change::Changed(entry)) = change else {
            panic!("expected a changed entry");
        };
        assert_eq!(entry.entry(), &observed);
        assert_eq!(entry.order_key().as_datetime(), order_key);
    }

    #[test]
    fn unchanged_entry_produces_no_change() {
        let observed_at = time("2026-07-20T12:00:00Z");
        let observed = Entry::builder().id(entry_id('a')).build();
        let current = SyndEntry::builder()
            .entry(observed.clone())
            .order_key(observed.resolve_order_key(observed_at))
            .build();

        let change = EntryChange::determine(Some(current), observed)
            .unwrap()
            .into_write(observed_at);

        assert_eq!(change, None);
    }

    fn entry_id(digit: char) -> EntryId {
        EntryId::parse(format!("synd:entry:v1:{}", digit.to_string().repeat(64))).unwrap()
    }

    fn time(value: &str) -> Time {
        value.parse().unwrap()
    }
}
