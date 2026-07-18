use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::{Annotated, EntryId, FeedMeta, FeedUrl};
use thiserror::Error;

use crate::{
    entry::EntryAttrs,
    subscription::{SubscriberId, Subscription},
};

/// Query for timeline items visible to one subscriber.
#[derive(Debug, Clone)]
pub struct TimelineItemsQuery {
    pub subscriber_id: SubscriberId,
    pub feed_url: Option<FeedUrl>,
    pub after: Option<TimelineItemCursor>,
    pub first: usize,
}

/// Opaque pagination cursor for timeline item ordering.
/// Carries the immutable order key of the last item on a page:
/// `(order_time, entry_id)` is a total order because `entry_id` is unique
/// and `order_time` is frozen at entry discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItemCursor {
    order_time: DateTime<Utc>,
    entry_id: EntryId,
}

impl TimelineItemCursor {
    pub fn new(order_time: DateTime<Utc>, entry_id: EntryId) -> Self {
        Self {
            order_time,
            entry_id,
        }
    }

    pub fn decode(value: &str) -> Result<Self, TimelineItemCursorError> {
        serde_json::from_str(value).map_err(TimelineItemCursorError::Invalid)
    }

    pub fn encode(&self) -> String {
        serde_json::to_string(self).expect("timeline item cursor serialization should not fail")
    }

    pub fn order_time(&self) -> DateTime<Utc> {
        self.order_time
    }

    pub fn entry_id(&self) -> &EntryId {
        &self.entry_id
    }
}

impl fmt::Display for TimelineItemCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

/// Error returned when decoding a timeline item cursor.
#[derive(Debug, Error)]
pub enum TimelineItemCursorError {
    #[error("invalid timeline item cursor: {0}")]
    Invalid(serde_json::Error),
}

/// GraphQL/query node assembled for one timeline item.
#[derive(Debug, Clone)]
pub struct TimelineItemNode {
    pub entry_id: EntryId,
    pub attrs: EntryAttrs,
    pub feed_meta: Annotated<FeedMeta>,
    pub subscription: Subscription,
    pub cursor: TimelineItemCursor,
}

/// Page of timeline items returned by a timeline query.
#[derive(Debug, Clone)]
pub struct TimelineItemsPage {
    pub nodes: Vec<TimelineItemNode>,
    pub has_next_page: bool,
    pub end_cursor: Option<TimelineItemCursor>,
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
    /// Insert or overwrite the item at its `(order_time, entry_id)` position.
    Upsert(Box<TimelineItemNode>),
    /// Remove the item identified by `entry_id`.
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
    fn timeline_item_cursor_roundtrips_as_opaque_string() {
        let cursor = TimelineItemCursor::new(
            Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap(),
            EntryId::parse(
                "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
        );

        assert_eq!(
            TimelineItemCursor::decode(&cursor.encode()).unwrap(),
            cursor
        );
    }
}
