use std::{cmp::Ordering, fmt};

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineItemCursor {
    order_time: DateTime<Utc>,
    feed_url: FeedUrl,
    entry_id: EntryId,
}

impl TimelineItemCursor {
    pub fn new(order_time: DateTime<Utc>, feed_url: FeedUrl, entry_id: EntryId) -> Self {
        Self {
            order_time,
            feed_url,
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

    pub fn feed_url(&self) -> &FeedUrl {
        &self.feed_url
    }

    pub fn entry_id(&self) -> &EntryId {
        &self.entry_id
    }
}

impl Ord for TimelineItemCursor {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .order_time
            .cmp(&self.order_time)
            .then_with(|| other.feed_url.as_str().cmp(self.feed_url.as_str()))
            .then_with(|| other.entry_id.as_str().cmp(self.entry_id.as_str()))
    }
}

impl PartialOrd for TimelineItemCursor {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use synd_feed::types::{EntryId, FeedUrl};

    use super::*;

    fn cursor(time: DateTime<Utc>, feed_url: &str, entry_id: &str) -> TimelineItemCursor {
        TimelineItemCursor::new(
            time,
            FeedUrl::parse(feed_url).unwrap(),
            EntryId::parse(entry_id).unwrap(),
        )
    }

    #[test]
    fn timeline_item_cursor_roundtrips_as_opaque_string() {
        let cursor = cursor(
            Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap(),
            "https://example.com/feed.xml",
            "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000001",
        );

        assert_eq!(
            TimelineItemCursor::decode(&cursor.encode()).unwrap(),
            cursor
        );
    }

    #[test]
    fn timeline_item_cursor_orders_by_display_order() {
        let newer = cursor(
            Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap(),
            "https://example.com/a.xml",
            "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000001",
        );
        let older = cursor(
            Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap(),
            "https://example.com/a.xml",
            "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000001",
        );
        let same_time_earlier_feed = cursor(
            Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap(),
            "https://example.com/b.xml",
            "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000001",
        );
        let same_identity_earlier_entry = cursor(
            Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap(),
            "https://example.com/a.xml",
            "synd:entry:v1:0000000000000000000000000000000000000000000000000000000000000002",
        );

        assert!(newer < older);
        assert!(same_time_earlier_feed < newer);
        assert!(same_identity_earlier_entry < newer);
    }
}
