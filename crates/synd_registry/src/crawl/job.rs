use std::fmt;

use chrono::{DateTime, Utc};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

/// One crawl accepted from the dispatch queue and handed to a worker.
///
/// Crawl jobs live only for the duration of one fetch; durable facts about
/// the run are recorded as crawl results keyed by `job_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlJob {
    pub job_id: CrawlJobId,
    pub feed_url: FeedUrl,
    pub trigger: CrawlJobTrigger,
    pub started_at: DateTime<Utc>,
}

impl CrawlJob {
    pub fn new(
        job_id: CrawlJobId,
        feed_url: FeedUrl,
        trigger: CrawlJobTrigger,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            job_id,
            feed_url,
            trigger,
            started_at,
        }
    }
}

/// Public identity for one crawl job.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CrawlJobId(String);

impl CrawlJobId {
    pub fn generate() -> Self {
        Self(Alphanumeric.sample_string(&mut rand::rng(), 24))
    }

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CrawlJobId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Reason a crawl job was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlJobTrigger {
    PeriodicDue,
    ManualRequest,
    RetryDue,
}

impl CrawlJobTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PeriodicDue => "periodic_due",
            Self::ManualRequest => "manual_request",
            Self::RetryDue => "retry_due",
        }
    }

    /// The worker queue lane that runs jobs with this trigger.
    pub fn queue_lane(self) -> CrawlJobQueueLane {
        match self {
            Self::ManualRequest => CrawlJobQueueLane::Manual,
            Self::RetryDue => CrawlJobQueueLane::Retry,
            Self::PeriodicDue => CrawlJobQueueLane::Default,
        }
    }
}

impl fmt::Display for CrawlJobTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Queue lane used to prioritize crawl job claiming.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlJobQueueLane {
    Default,
    Manual,
    Retry,
}

impl CrawlJobQueueLane {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Manual => "manual",
            Self::Retry => "retry",
        }
    }
}

impl fmt::Display for CrawlJobQueueLane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
