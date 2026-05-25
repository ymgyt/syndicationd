use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistryEvent {
    TimelineChanged(TimelineChanged),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineChanged {
    pub changed_at: DateTime<Utc>,
    pub affected_feeds: AffectedFeeds,
}

impl TimelineChanged {
    pub fn for_feed(feed_url: FeedUrl, changed_at: DateTime<Utc>) -> Self {
        Self {
            changed_at,
            affected_feeds: AffectedFeeds::Known(vec![feed_url]),
        }
    }

    pub fn for_feeds(feed_urls: Vec<FeedUrl>, changed_at: DateTime<Utc>) -> Self {
        Self {
            changed_at,
            affected_feeds: AffectedFeeds::Known(feed_urls),
        }
    }

    pub fn unknown(changed_at: DateTime<Utc>) -> Self {
        Self {
            changed_at,
            affected_feeds: AffectedFeeds::Unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AffectedFeeds {
    Unknown,
    Known(Vec<FeedUrl>),
}
