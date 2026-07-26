use serde::Deserialize;
use synd_feed::types::{FeedUrl, Time};

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "__typename")]
pub enum FeedEvent {
    TimelineChanged(TimelineChangeEvent),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineChangeEvent {
    pub changed_at: Time,
    pub affected_feeds: Option<Vec<FeedUrl>>,
}

#[cfg(test)]
mod tests {
    use core::assert_matches;

    use super::FeedEvent;

    #[test]
    fn decodes_timeline_changed_feed_event() {
        let event: FeedEvent = serde_json::from_value(serde_json::json!({
            "__typename": "TimelineChanged",
            "changedAt": "2026-06-13T00:00:00Z",
            "affectedFeeds": ["https://example.com/feed.xml"]
        }))
        .unwrap();

        let FeedEvent::TimelineChanged(event) = event;
        assert_eq!(event.changed_at.to_rfc3339(), "2026-06-13T00:00:00+00:00");
        assert_matches!(event.affected_feeds, Some(feeds) if feeds.len() == 1);
    }
}
