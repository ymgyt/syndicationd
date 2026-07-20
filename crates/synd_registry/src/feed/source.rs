use bon::Builder;
use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::crawl::{blob::BlobRef, job::CrawlJobId};

/// Accepted crawl body used to derive the current feed state.
#[derive(Debug, Clone, Builder, PartialEq, Eq)]
pub struct FeedSource {
    pub feed_url: FeedUrl,
    pub crawl_job_id: CrawlJobId,
    pub body_blob: BlobRef,
    pub seen_at: DateTime<Utc>,
}
