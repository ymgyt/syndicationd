use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::{Annotated, EntryId, FeedMeta};
use thiserror::Error;

use crate::{
    entry::EntryAttrs,
    subscription::{SubscriberId, Subscription},
};

/// Query for timeline entries visible to one subscriber.
#[derive(Debug, Clone)]
pub struct TimelineEntriesQuery {
    pub subscriber_id: SubscriberId,
    pub after: Option<TimelineEntryCursor>,
    pub first: usize,
}

/// Opaque pagination cursor for timeline entry ordering.
/// Carries the immutable order key of the last entry on a page:
/// `(order_time, entry_id)` is a total order because `entry_id` is unique
/// and `order_time` is frozen at entry discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEntryCursor {
    order_time: DateTime<Utc>,
    entry_id: EntryId,
}

impl TimelineEntryCursor {
    pub fn new(order_time: DateTime<Utc>, entry_id: EntryId) -> Self {
        Self {
            order_time,
            entry_id,
        }
    }

    pub fn decode(value: &str) -> Result<Self, TimelineEntryCursorError> {
        serde_json::from_str(value).map_err(TimelineEntryCursorError::Invalid)
    }

    pub fn encode(&self) -> String {
        serde_json::to_string(self).expect("timeline entry cursor serialization should not fail")
    }

    pub fn order_time(&self) -> DateTime<Utc> {
        self.order_time
    }

    pub fn entry_id(&self) -> &EntryId {
        &self.entry_id
    }
}

impl fmt::Display for TimelineEntryCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

/// Error returned when decoding a timeline entry cursor.
#[derive(Debug, Error)]
pub enum TimelineEntryCursorError {
    #[error("invalid timeline entry cursor: {0}")]
    Invalid(serde_json::Error),
}

/// GraphQL/query node assembled for one timeline entry.
#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub entry_id: EntryId,
    pub attrs: EntryAttrs,
    pub feed_meta: Annotated<FeedMeta>,
    pub subscription: Subscription,
    pub cursor: TimelineEntryCursor,
}

/// Page of timeline entries returned by a timeline query.
#[derive(Debug, Clone)]
pub struct TimelineEntriesPage {
    pub nodes: Vec<TimelineEntry>,
    pub has_next_page: bool,
    pub end_cursor: Option<TimelineEntryCursor>,
    /// Change seq this snapshot reflects, read in the same transaction.
    /// Clients start syncing changes from here.
    pub seq: i64,
}

/// Query for timeline changes observed after a known seq.
#[derive(Debug, Clone)]
pub struct TimelineChangesQuery {
    pub subscriber_id: SubscriberId,
    pub since: i64,
    pub limit: usize,
}

/// One timeline change, applied by clients in seq order.
#[derive(Debug, Clone)]
pub enum TimelineChange {
    /// Insert or overwrite the entry at its `(order_time, entry_id)` position.
    Upsert(Box<TimelineEntry>),
    /// Remove the entry identified by `entry_id`.
    Remove { entry_id: EntryId },
}

/// Page of timeline changes for incremental sync.
#[derive(Debug, Clone)]
pub struct TimelineChangesPage {
    pub changes: Vec<TimelineChange>,
    /// Seq the client remembers after applying this page.
    /// Equals the server position when `has_more` is false.
    pub seq: i64,
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use synd_feed::types::EntryId;

    use super::*;

    #[test]
    fn timeline_entry_cursor_roundtrips_as_opaque_string() {
        let cursor = TimelineEntryCursor::new(
            Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap(),
            EntryId::parse(
                "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
        );

        assert_eq!(
            TimelineEntryCursor::decode(&cursor.encode()).unwrap(),
            cursor
        );
    }
}
