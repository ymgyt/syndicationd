use bon::Builder;
use serde::{Deserialize, Serialize};
use synd_feed::{
    entry::EntryId,
    types::{Category, FeedUrl, Requirement, Time, TimelineName},
};

use super::PageInfo;

pub use uuid::Uuid as TimelineId;

#[derive(Debug, Deserialize)]
pub struct Timeline {
    pub id: TimelineId,
    pub name: TimelineName,
}

#[derive(Debug, Serialize)]
pub struct CreateTimelineInput {
    pub name: TimelineName,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTimelinePayload {
    pub timeline_id: TimelineId,
}

/// Request one page of entries from a timeline.
#[derive(Debug, Builder, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntriesRequest {
    timeline_id: TimelineId,
    first: i64,
    after: Option<String>,
    /// Maximum Unicode display width per excerpt. Omit to use the server default.
    #[serde(skip_serializing_if = "Option::is_none")]
    max_unicode_width: Option<u16>,
}

impl TimelineEntriesRequest {
    pub fn timeline_id(&self) -> TimelineId {
        self.timeline_id
    }

    pub fn first(&self) -> i64 {
        self.first
    }

    pub fn after(&self) -> Option<&str> {
        self.after.as_deref()
    }

    pub fn max_unicode_width(&self) -> Option<u16> {
        self.max_unicode_width
    }
}

/// Entry as it appears on one timeline: display position + content.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntry {
    pub order_time: Time,
    pub entry: Entry,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntryConnection {
    pub nodes: Vec<TimelineEntry>,
    pub page_info: PageInfo,
    /// Change seq this page reflects. The client syncs changes from here
    pub seq: i64,
}

/// Page of timeline changes for incremental sync, ordered by seq.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineChangesPayload {
    pub changes: Vec<TimelineChange>,
    /// Seq the client remembers after applying this page
    pub seq: i64,
    pub has_more: bool,
}

/// One timeline change, applied in seq order.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "__typename")]
pub enum TimelineChange {
    /// Insert or overwrite the entry at its `(order_time, entry.id)` position
    #[serde(rename = "TimelineChangeUpsert", rename_all = "camelCase")]
    Upsert { timeline_entry: Box<TimelineEntry> },
    /// Remove the entry identified by `entry_id`
    #[serde(rename = "TimelineChangeRemove", rename_all = "camelCase")]
    Remove { entry_id: EntryId },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: EntryId,
    pub title: Option<String>,
    pub published: Option<Time>,
    pub updated: Option<Time>,
    pub website_url: Option<String>,
    pub excerpt: Option<EntryExcerpt>,
    pub feed: FeedMeta,
}

#[derive(Debug, Clone, Deserialize)]
#[cfg_attr(any(test, feature = "fake"), derive(fake::Dummy))]
pub struct EntryExcerpt {
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FeedMeta {
    pub title: Option<String>,
    pub url: FeedUrl,
    #[serde(default, with = "super::requirement")]
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}
