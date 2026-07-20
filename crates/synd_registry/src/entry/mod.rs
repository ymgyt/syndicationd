use std::collections::{HashMap, HashSet};

use bon::Builder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::{
    Content as SyndFeedContent, Entry as SyndFeedEntry, EntryId, Feed as SyndFeed, FeedType,
    FeedUrl, Text as SyndFeedText,
};

use crate::feed::FeedSource;

mod projection;

pub use projection::{EntryProj, EntryProjInput};

/// Upper bound of the summary materialized from content when the feed
/// declares no summary. Keeps hot reads small; the full content is stored
/// separately and never travels with list queries.
const SUMMARY_FALLBACK_MAX_BYTES: usize = 4 * 1024;

/// A registry entry entity recognized by synd.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct Entry {
    pub id: EntryId,
    pub feed_url: FeedUrl,
    pub attrs: EntryAttrs,
    /// Full entry content. Kept out of `attrs` so hot queries do not carry it.
    pub content: Option<String>,
    pub order_key: EntryOrderKey,
}

impl Entry {
    fn discover(source: &FeedSource, appearance: EntryAppearance) -> Self {
        let order_key = EntryOrderKey::resolve(&appearance.attrs, source.seen_at);
        Self::builder()
            .id(appearance.id)
            .feed_url(source.feed_url.clone())
            .attrs(appearance.attrs)
            .maybe_content(appearance.content)
            .order_key(order_key)
            .build()
    }

    /// Reconciles a new appearance with this stored entry. Returns `None`
    /// when nothing observable changed, so unchanged entries cost no write.
    fn reconcile(self, appearance: EntryAppearance) -> Option<EntryChange> {
        if self.attrs == appearance.attrs && self.content == appearance.content {
            return None;
        }
        let entry = Self::builder()
            .id(self.id)
            .feed_url(self.feed_url)
            .attrs(appearance.attrs)
            .maybe_content(appearance.content)
            // order_key is immutable: keep the key resolved at discovery even
            // if the feed later changes published/updated timestamps
            .order_key(self.order_key)
            .build();
        Some(EntryChange::Changed(entry))
    }
}

/// Feed-derived entry attributes owned by the registry current model.
///
/// `summary` is materialized at observation: when the feed declares none,
/// a truncated slice of the content substitutes, so readers never need the
/// full content for list rendering.
#[derive(Debug, Clone, Builder, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EntryAttrs {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub website_url: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl EntryAttrs {
    fn from_feed_entry(feed_type: FeedType, entry: &SyndFeedEntry) -> Self {
        let summary = entry
            .summary()
            .map(SyndFeedText::content)
            .map(str::to_owned)
            .or_else(|| {
                entry
                    .content()
                    .and_then(SyndFeedContent::body)
                    .map(summary_fallback)
            });
        Self::builder()
            .maybe_title(entry.title().map(SyndFeedText::content).map(str::to_owned))
            .maybe_summary(summary)
            .maybe_website_url(entry.website_url(feed_type).map(str::to_owned))
            .maybe_published_at(entry.published())
            .maybe_updated_at(entry.updated())
            .build()
    }
}

fn summary_fallback(content: &str) -> String {
    let mut end = SUMMARY_FALLBACK_MAX_BYTES.min(content.len());
    while !content.is_char_boundary(end) {
        end -= 1;
    }
    content[..end].to_owned()
}

/// Order key that places an entry in the canonical entry order.
/// Resolved once when the entry is discovered and frozen afterwards so that
/// display order and pagination cursors stay stable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryOrderKey(DateTime<Utc>);

impl EntryOrderKey {
    pub fn from_datetime(time: DateTime<Utc>) -> Self {
        Self(time)
    }

    pub fn resolve(attrs: &EntryAttrs, first_seen_at: DateTime<Utc>) -> Self {
        attrs
            .published_at
            .or(attrs.updated_at)
            .map_or(Self(first_seen_at), Self::from_datetime)
    }

    pub fn as_datetime(self) -> DateTime<Utc> {
        self.0
    }
}

/// An entry's appearance in one accepted feed source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryAppearance {
    id: EntryId,
    attrs: EntryAttrs,
    content: Option<String>,
}

impl EntryAppearance {
    fn from_feed_entry(feed_type: FeedType, entry: &SyndFeedEntry) -> Self {
        let attrs = EntryAttrs::from_feed_entry(feed_type, entry);
        Self {
            id: entry.id().clone(),
            attrs,
            content: entry
                .content()
                .and_then(SyndFeedContent::body)
                .map(str::to_owned),
        }
    }

    fn id(&self) -> &EntryId {
        &self.id
    }
}

/// Entry appearances contained in one accepted feed source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryAppearances {
    entries: Vec<EntryAppearance>,
}

impl EntryAppearances {
    pub fn from_feed(feed: &SyndFeed) -> Self {
        let feed_type = feed.meta().r#type();
        let mut seen = HashSet::new();
        let entries = feed
            .entries()
            .filter_map(|entry| {
                let appearance = EntryAppearance::from_feed_entry(feed_type, entry);
                seen.insert(appearance.id.clone()).then_some(appearance)
            })
            .collect();
        Self { entries }
    }

    pub fn ids(&self) -> Vec<EntryId> {
        self.entries
            .iter()
            .map(|appearance| appearance.id.clone())
            .collect()
    }

    fn into_entries(self) -> Vec<EntryAppearance> {
        self.entries
    }
}

/// Existing registry entries for one feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntrySet {
    feed_url: FeedUrl,
    entries: HashMap<EntryId, Entry>,
}

impl EntrySet {
    pub fn new(feed_url: FeedUrl, entries: Vec<Entry>) -> Self {
        debug_assert!(entries.iter().all(|entry| entry.feed_url == feed_url));
        let entries = entries
            .into_iter()
            .map(|entry| (entry.id.clone(), entry))
            .collect();
        Self { feed_url, entries }
    }

    pub fn empty(feed_url: FeedUrl) -> Self {
        Self::new(feed_url, Vec::new())
    }

    fn remove(&mut self, id: &EntryId) -> Option<Entry> {
        self.entries.remove(id)
    }
}

/// Compares entry appearances from a feed source with existing registry entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryReconciliation {
    source: FeedSource,
    appearances: EntryAppearances,
    existing: EntrySet,
}

impl EntryReconciliation {
    pub fn new(source: FeedSource, appearances: EntryAppearances, existing: EntrySet) -> Self {
        debug_assert_eq!(source.feed_url, existing.feed_url);
        Self {
            source,
            appearances,
            existing,
        }
    }

    /// Diffs appearances against stored entries. Unchanged entries produce
    /// no change at all.
    pub fn reconcile(mut self) -> EntryChanges {
        let mut changes = Vec::new();
        for appearance in self.appearances.into_entries() {
            let change = match self.existing.remove(appearance.id()) {
                Some(entry) => entry.reconcile(appearance),
                None => Some(EntryChange::Discovered(Entry::discover(
                    &self.source,
                    appearance,
                ))),
            };
            changes.extend(change);
        }
        EntryChanges::new(changes)
    }
}

/// Result of reconciling one entry appearance with registry state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryChange {
    Discovered(Entry),
    Changed(Entry),
}

impl EntryChange {
    pub fn entry(&self) -> &Entry {
        match self {
            Self::Discovered(entry) | Self::Changed(entry) => entry,
        }
    }
}

/// Entry changes to persist after one reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryChanges {
    changes: Vec<EntryChange>,
}

impl EntryChanges {
    pub fn new(changes: Vec<EntryChange>) -> Self {
        Self { changes }
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &EntryChange> {
        self.changes.iter()
    }

    pub fn into_changes(self) -> Vec<EntryChange> {
        self.changes
    }
}
