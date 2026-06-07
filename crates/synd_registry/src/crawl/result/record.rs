use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::crawl::{job::CrawlJobId, result::CrawlResultDetail};

/// Database reference to one persisted crawl result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrawlResultRef {
    pk: i64,
}

impl CrawlResultRef {
    pub fn new(pk: i64) -> Self {
        Self { pk }
    }

    pub fn pk(self) -> i64 {
        self.pk
    }
}

/// Command to persist the immutable facts for one finished crawl job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCrawlResultCommand {
    pub record: CrawlResultRecord,
    pub detail: CrawlResultDetail,
}

impl RecordCrawlResultCommand {
    pub fn new(record: CrawlResultRecord, detail: CrawlResultDetail) -> Self {
        Self { record, detail }
    }
}

/// Thin crawl-result record shared by every result detail shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlResultRecord {
    pub job_id: CrawlJobId,
    pub feed_url: FeedUrl,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

impl CrawlResultRecord {
    pub fn new(
        job_id: CrawlJobId,
        feed_url: FeedUrl,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
    ) -> Self {
        Self {
            job_id,
            feed_url,
            started_at,
            finished_at,
        }
    }
}
