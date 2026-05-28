use std::fmt;

use feed_rs::model as feedrs;

use super::{EntryId, EntryIdError, FeedType, FeedUrl, Time, link};

#[derive(Clone)]
pub struct Entry {
    id: EntryId,
    entry: feedrs::Entry,
}

impl Entry {
    pub fn id(&self) -> EntryId {
        self.id.clone()
    }

    pub fn id_ref(&self) -> &EntryId {
        &self.id
    }

    pub fn title(&self) -> Option<&str> {
        self.entry.title.as_ref().map(|text| text.content.as_str())
    }

    pub fn updated(&self) -> Option<Time> {
        self.entry.updated
    }

    pub fn published(&self) -> Option<Time> {
        self.entry.published
    }

    pub fn summary(&self) -> Option<&str> {
        self.entry
            .summary
            .as_ref()
            .map(|text| text.content.as_str())
    }

    pub fn content(&self) -> Option<&str> {
        self.entry
            .content
            .as_ref()
            .and_then(|content| content.body.as_deref())
    }

    pub fn website_url(&self, feed_type: FeedType) -> Option<&str> {
        link::find_website_url(feed_type, &self.entry.links)
    }

    /// Return approximate entry bytes size
    pub fn approximate_size(&self) -> usize {
        let content_size = self
            .entry
            .content
            .as_ref()
            .and_then(|content| content.body.as_deref())
            .map_or(0, str::len);

        let summary_size = self
            .entry
            .summary
            .as_ref()
            .map_or(0, |summary| summary.content.len());

        content_size + summary_size
    }

    pub(super) fn from_feed_rs(
        feed_url: &FeedUrl,
        feed_type: FeedType,
        entry: feedrs::Entry,
    ) -> Result<Self, EntryIdError> {
        Ok(Self {
            id: EntryId::from_feed_entry(feed_url, feed_type, &entry)?,
            entry,
        })
    }
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("id", &self.id)
            .field("title", &self.title())
            .field("updated", &self.updated())
            .field("published", &self.published())
            .finish_non_exhaustive()
    }
}
