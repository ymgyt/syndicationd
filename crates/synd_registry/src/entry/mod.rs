use std::collections::{HashMap, HashSet};

use bon::Builder;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::{Entry as SyndFeedEntry, EntryId, Feed as SyndFeed, FeedType, FeedUrl};

use crate::{
    crawl::{job::CrawlJobId, result::CrawlResultRef},
    feed::FeedSource,
};

mod projection;

pub use projection::{EntryProj, EntryProjectionInput, EntryProjectionScope};

/// A registry entry entity recognized by synd.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct Entry {
    pub id: EntryId,
    pub feed_url: FeedUrl,
    pub attrs: EntryAttrs,
    pub order_key: EntryOrderKey,
    pub lifecycle: EntryLifecycle,
    pub source: EntrySourceRef,
}

impl Entry {
    fn discover(source: &FeedSource, appearance: EntryAppearance) -> Self {
        let order_key = EntryOrderKey::resolve(
            &appearance.attrs,
            EntryOrderFallback::FirstSeenAt(source.seen_at),
        );
        Self::builder()
            .id(appearance.id)
            .feed_url(source.feed_url.clone())
            .attrs(appearance.attrs)
            .order_key(order_key)
            .lifecycle(EntryLifecycle::first_seen(source.seen_at))
            .source(EntrySourceRef::from(source))
            .build()
    }

    fn reconcile(self, source: &FeedSource, appearance: EntryAppearance) -> EntryChange {
        let order_key = EntryOrderKey::resolve(
            &appearance.attrs,
            EntryOrderFallback::Existing(self.order_key),
        );
        let changed = self.attrs != appearance.attrs || self.order_key != order_key;
        let entry = Self::builder()
            .id(self.id)
            .feed_url(self.feed_url)
            .attrs(appearance.attrs)
            .order_key(order_key)
            .lifecycle(self.lifecycle.seen_again(source.seen_at))
            .source(EntrySourceRef::from(source))
            .build();
        if changed {
            EntryChange::Changed(entry)
        } else {
            EntryChange::AlreadySeen(entry)
        }
    }
}

/// Feed-derived entry attributes owned by the registry current model.
#[derive(Debug, Clone, Builder, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EntryAttrs {
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: Option<String>,
    pub website_url: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl EntryAttrs {
    fn from_feed_entry(feed_type: FeedType, entry: &SyndFeedEntry) -> Self {
        Self::builder()
            .maybe_title(entry.title().map(str::to_owned))
            .maybe_summary(entry.summary().map(str::to_owned))
            .maybe_content(entry.content().map(str::to_owned))
            .maybe_website_url(entry.website_url(feed_type).map(str::to_owned))
            .maybe_published_at(entry.published())
            .maybe_updated_at(entry.updated())
            .build()
    }
}

/// Interpreted order key for placing an entry in timeline projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntryOrderKey(DateTime<Utc>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryOrderFallback {
    FirstSeenAt(DateTime<Utc>),
    Existing(EntryOrderKey),
}

impl EntryOrderKey {
    pub fn from_datetime(time: DateTime<Utc>) -> Self {
        Self(time)
    }

    pub fn resolve(attrs: &EntryAttrs, fallback: EntryOrderFallback) -> Self {
        attrs
            .published_at
            .or(attrs.updated_at)
            .map_or_else(|| fallback.into_order_key(), Self::from_datetime)
    }

    pub fn as_datetime(self) -> DateTime<Utc> {
        self.0
    }
}

impl EntryOrderFallback {
    fn into_order_key(self) -> EntryOrderKey {
        match self {
            Self::FirstSeenAt(seen_at) => EntryOrderKey::from_datetime(seen_at),
            Self::Existing(order_key) => order_key,
        }
    }
}

/// Lifecycle timestamps for a registry entry.
#[derive(Debug, Clone, Copy, Builder, PartialEq, Eq)]
pub struct EntryLifecycle {
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl EntryLifecycle {
    fn first_seen(now: DateTime<Utc>) -> Self {
        Self::builder()
            .first_seen_at(now)
            .last_seen_at(now)
            .updated_at(now)
            .build()
    }

    fn seen_again(self, now: DateTime<Utc>) -> Self {
        Self::builder()
            .first_seen_at(self.first_seen_at)
            .last_seen_at(now)
            .updated_at(now)
            .build()
    }
}

/// Crawl source that last supplied a registry entry.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct EntrySourceRef {
    pub crawl_job_id: CrawlJobId,
    pub result_ref: CrawlResultRef,
}

impl From<&FeedSource> for EntrySourceRef {
    fn from(source: &FeedSource) -> Self {
        Self::builder()
            .crawl_job_id(source.crawl_job_id.clone())
            .result_ref(source.result_ref)
            .build()
    }
}

/// An entry's appearance in one accepted feed source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryAppearance {
    id: EntryId,
    attrs: EntryAttrs,
}

impl EntryAppearance {
    fn from_feed_entry(feed_type: FeedType, entry: &SyndFeedEntry) -> Self {
        let attrs = EntryAttrs::from_feed_entry(feed_type, entry);
        Self {
            id: entry.id(),
            attrs,
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

    pub fn reconcile(mut self) -> EntryChanges {
        let mut changes = Vec::new();
        for appearance in self.appearances.into_entries() {
            let change = match self.existing.remove(appearance.id()) {
                Some(entry) => entry.reconcile(&self.source, appearance),
                None => EntryChange::Discovered(Entry::discover(&self.source, appearance)),
            };
            changes.push(change);
        }
        EntryChanges::new(changes)
    }
}

/// Result of reconciling one entry appearance with registry state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryChange {
    Discovered(Entry),
    Changed(Entry),
    AlreadySeen(Entry),
}

impl EntryChange {
    pub fn entry(&self) -> &Entry {
        match self {
            Self::Discovered(entry) | Self::Changed(entry) | Self::AlreadySeen(entry) => entry,
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
