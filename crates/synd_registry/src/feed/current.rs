use bon::Builder;
use chrono::{DateTime, Utc};
use synd_feed::types::{FeedMeta, FeedUrl};

use crate::{
    crawl::{blob::BlobRef, job::CrawlJobId, result::CrawlResultRef},
    event::{Event, FeedChangedEvent, FeedDiscoveredEvent},
};

/// Source crawl result used to derive the current feed state.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct FeedSource {
    pub feed_url: FeedUrl,
    pub crawl_job_id: CrawlJobId,
    pub result_ref: CrawlResultRef,
    pub body_blob: BlobRef,
    pub seen_at: DateTime<Utc>,
}

/// Command to replace the current feed state with the latest parsed result.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct UpsertFeedCommand {
    pub source: FeedSource,
    pub meta: FeedMeta,
}

/// Result of applying one parsed feed to the current feed state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertFeedOutcome {
    Discovered,
    Changed,
    Unchanged,
}

impl UpsertFeedOutcome {
    /// Returns the feed lifecycle event represented by this write outcome.
    pub fn into_event(self, source: &FeedSource) -> Option<Event> {
        match self {
            Self::Discovered => Some(
                FeedDiscoveredEvent::new(source.feed_url.clone(), source.crawl_job_id.clone())
                    .into(),
            ),
            Self::Changed => Some(
                FeedChangedEvent::new(source.feed_url.clone(), source.crawl_job_id.clone()).into(),
            ),
            Self::Unchanged => None,
        }
    }
}
