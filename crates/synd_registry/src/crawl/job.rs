use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

/// Runtime crawl job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlJob {
    pub job_id: CrawlJobId,
    pub feed_url: FeedUrl,
    pub state: CrawlJobState,
    pub trigger: CrawlJobTrigger,
    pub queue: CrawlJobQueueLane,
    pub priority: i64,
    pub run_after: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CrawlJob {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_id: CrawlJobId,
        feed_url: FeedUrl,
        state: CrawlJobState,
        trigger: CrawlJobTrigger,
        queue: CrawlJobQueueLane,
        priority: i64,
        run_after: DateTime<Utc>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            job_id,
            feed_url,
            state,
            trigger,
            queue,
            priority,
            run_after,
            created_at,
            updated_at,
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

/// Current lifecycle state of one crawl job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlJobState {
    Pending,
    Running,
    Finished,
    Cancelled,
}

impl CrawlJobState {
    pub const PENDING: &'static str = "pending";
    pub const RUNNING: &'static str = "running";
    pub const FINISHED: &'static str = "finished";
    pub const CANCELLED: &'static str = "cancelled";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => Self::PENDING,
            Self::Running => Self::RUNNING,
            Self::Finished => Self::FINISHED,
            Self::Cancelled => Self::CANCELLED,
        }
    }
}

impl fmt::Display for CrawlJobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CrawlJobState {
    type Err = UnknownCrawlJobValue;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            Self::PENDING => Ok(Self::Pending),
            Self::RUNNING => Ok(Self::Running),
            Self::FINISHED => Ok(Self::Finished),
            Self::CANCELLED => Ok(Self::Cancelled),
            value => Err(UnknownCrawlJobValue::new("state", value)),
        }
    }
}

/// Reason a crawl job was requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlJobTrigger {
    TargetChanged,
    PeriodicDue,
    ManualRequest,
    RetryDue,
}

impl CrawlJobTrigger {
    pub const TARGET_CHANGED: &'static str = "target_changed";
    pub const PERIODIC_DUE: &'static str = "periodic_due";
    pub const MANUAL_REQUEST: &'static str = "manual_request";
    pub const RETRY_DUE: &'static str = "retry_due";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::TargetChanged => Self::TARGET_CHANGED,
            Self::PeriodicDue => Self::PERIODIC_DUE,
            Self::ManualRequest => Self::MANUAL_REQUEST,
            Self::RetryDue => Self::RETRY_DUE,
        }
    }
}

impl fmt::Display for CrawlJobTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CrawlJobTrigger {
    type Err = UnknownCrawlJobValue;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            Self::TARGET_CHANGED => Ok(Self::TargetChanged),
            Self::PERIODIC_DUE => Ok(Self::PeriodicDue),
            Self::MANUAL_REQUEST => Ok(Self::ManualRequest),
            Self::RETRY_DUE => Ok(Self::RetryDue),
            value => Err(UnknownCrawlJobValue::new("trigger", value)),
        }
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
    pub const DEFAULT: &'static str = "default";
    pub const MANUAL: &'static str = "manual";
    pub const RETRY: &'static str = "retry";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => Self::DEFAULT,
            Self::Manual => Self::MANUAL,
            Self::Retry => Self::RETRY,
        }
    }
}

impl fmt::Display for CrawlJobQueueLane {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CrawlJobQueueLane {
    type Err = UnknownCrawlJobValue;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            Self::DEFAULT => Ok(Self::Default),
            Self::MANUAL => Ok(Self::Manual),
            Self::RETRY => Ok(Self::Retry),
            value => Err(UnknownCrawlJobValue::new("queue", value)),
        }
    }
}

/// Error returned when stored crawl job values are unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCrawlJobValue {
    field: &'static str,
    value: String,
}

impl UnknownCrawlJobValue {
    fn new(field: &'static str, value: impl Into<String>) -> Self {
        Self {
            field,
            value: value.into(),
        }
    }
}

impl fmt::Display for UnknownCrawlJobValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown crawl job {}: {}", self.field, self.value)
    }
}

impl std::error::Error for UnknownCrawlJobValue {}
