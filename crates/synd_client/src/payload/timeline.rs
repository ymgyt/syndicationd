use std::collections::BTreeMap;

use bon::Builder;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use synd_feed::{
    entry::{EntryId, SourceCategory},
    types::{FeedUrl, SubscriptionLabels, Time, TimelineName},
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
    /// Include entries whose order time is at or after this instant.
    #[serde(skip_serializing_if = "Option::is_none")]
    since: Option<Time>,
    /// Include entries whose order time is before this instant.
    #[serde(skip_serializing_if = "Option::is_none")]
    until: Option<Time>,
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

    pub(crate) fn has_time_range(&self) -> bool {
        self.since.is_some() || self.until.is_some()
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
    /// Categories declared by the publisher of this entry.
    #[serde(
        default,
        rename = "labels",
        deserialize_with = "source_categories_from_labels"
    )]
    pub source_categories: Vec<SourceCategory>,
    pub feed: FeedMeta,
}

// The current GraphQL API returns publisher categories inside the entry's labels.
fn source_categories_from_labels<'de, D>(deserializer: D) -> Result<Vec<SourceCategory>, D::Error>
where
    D: Deserializer<'de>,
{
    let mut labels = BTreeMap::<String, Value>::deserialize(deserializer)?;
    let Some(categories) = labels.remove(SourceCategory::LABEL_KEY) else {
        return Ok(Vec::new());
    };
    Ok(serde_json::from_value(categories).unwrap_or_default())
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
    pub labels: SubscriptionLabels,
}
