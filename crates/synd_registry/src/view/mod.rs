use std::{cmp::Ordering, fmt};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::{Annotated, Entry, FeedMeta, FeedUrl};
use thiserror::Error;

use crate::{subscriber::SubscriberId, subscription::Subscription};

#[derive(Debug, Clone)]
pub struct SubscriptionsQuery {
    pub subscriber_id: SubscriberId,
    pub after: Option<String>,
    pub first: usize,
}

#[derive(Debug, Clone)]
pub struct EntriesQuery {
    pub subscriber_id: SubscriberId,
    pub after: Option<EntryCursor>,
    pub first: usize,
}

#[derive(Debug, Clone)]
pub struct Subscriptions {
    pub subscriptions: Vec<Subscription>,
    pub has_next_page: bool,
    pub end_cursor: Option<String>,
}

impl Subscriptions {
    pub fn from_subscriptions(
        subscriptions: Vec<Subscription>,
        has_next_page: bool,
        end_cursor: Option<String>,
    ) -> Self {
        Self {
            subscriptions,
            has_next_page,
            end_cursor,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryCursor {
    sort_time: Option<DateTime<Utc>>,
    feed_url: FeedUrl,
    entry_id: String,
    ordinal: usize,
}

impl EntryCursor {
    pub fn new(
        sort_time: Option<DateTime<Utc>>,
        feed_url: FeedUrl,
        entry_id: String,
        ordinal: usize,
    ) -> Self {
        Self {
            sort_time,
            feed_url,
            entry_id,
            ordinal,
        }
    }

    pub fn for_entry(feed_url: FeedUrl, entry: &Entry, ordinal: usize) -> Self {
        Self::new(
            entry.published().or(entry.updated()),
            feed_url,
            entry.id().to_string(),
            ordinal,
        )
    }

    pub fn decode(value: &str) -> Result<Self, EntryCursorError> {
        serde_json::from_str(value).map_err(EntryCursorError::Invalid)
    }

    pub fn encode(&self) -> String {
        serde_json::to_string(self).expect("entry cursor serialization should not fail")
    }

    pub fn sort_cmp(&self, other: &Self) -> Ordering {
        match (self.sort_time, other.sort_time) {
            (Some(a), Some(b)) => b.cmp(&a),
            (None, Some(_)) => Ordering::Greater,
            (Some(_), None) => Ordering::Less,
            (None, None) => Ordering::Equal,
        }
        .then_with(|| self.feed_url.as_str().cmp(other.feed_url.as_str()))
        .then_with(|| self.entry_id.cmp(&other.entry_id))
        .then_with(|| self.ordinal.cmp(&other.ordinal))
    }

    pub fn is_after(&self, cursor: &Self) -> bool {
        self.sort_cmp(cursor).is_gt()
    }
}

#[derive(Debug, Error)]
pub enum EntryCursorError {
    #[error("invalid entry cursor: {0}")]
    Invalid(serde_json::Error),
}

impl fmt::Display for EntryCursor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.encode())
    }
}

#[derive(Debug, Clone)]
pub struct EntryView {
    pub entry: Entry,
    pub feed_meta: Annotated<FeedMeta>,
    pub cursor: EntryCursor,
}

#[derive(Debug, Clone)]
pub struct EntriesPage {
    pub nodes: Vec<EntryView>,
    pub has_next_page: bool,
    pub end_cursor: Option<EntryCursor>,
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use synd_feed::types::FeedUrl;

    use super::*;

    fn cursor(
        time: Option<DateTime<Utc>>,
        feed_url: &str,
        entry_id: &str,
        ordinal: usize,
    ) -> EntryCursor {
        EntryCursor::new(
            time,
            FeedUrl::parse(feed_url).unwrap(),
            entry_id.to_owned(),
            ordinal,
        )
    }

    #[test]
    fn entry_cursor_roundtrips_as_opaque_string() {
        let cursor = cursor(
            Some(Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap()),
            "https://example.com/feed.xml",
            "entry-1",
            0,
        );

        assert_eq!(EntryCursor::decode(&cursor.encode()).unwrap(), cursor);
    }

    #[test]
    fn entry_cursor_orders_by_time_feed_url_and_entry_id() {
        let newer = cursor(
            Some(Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap()),
            "https://example.com/a.xml",
            "1",
            0,
        );
        let older = cursor(
            Some(Utc.with_ymd_and_hms(2026, 5, 23, 12, 0, 0).unwrap()),
            "https://example.com/a.xml",
            "1",
            0,
        );
        let same_time_later_feed = cursor(
            Some(Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap()),
            "https://example.com/b.xml",
            "1",
            0,
        );
        let same_identity_later_ordinal = cursor(
            Some(Utc.with_ymd_and_hms(2026, 5, 24, 12, 0, 0).unwrap()),
            "https://example.com/a.xml",
            "1",
            1,
        );
        let missing_time = cursor(None, "https://example.com/a.xml", "1", 0);

        assert!(older.is_after(&newer));
        assert!(same_time_later_feed.is_after(&newer));
        assert!(same_identity_later_ordinal.is_after(&newer));
        assert!(missing_time.is_after(&older));
    }
}
