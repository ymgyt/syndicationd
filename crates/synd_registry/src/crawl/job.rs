use std::{fmt, str::FromStr};

use chrono::{DateTime, Utc};
use rand::distr::{Alphanumeric, SampleString};
use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

/// Current queue-wide facts visible to one scheduler run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrawlQueueSnapshot {
    pub pending_count: u64,
    pub running_count: u64,
}

impl CrawlQueueSnapshot {
    pub fn new(pending_count: u64, running_count: u64) -> Self {
        Self {
            pending_count,
            running_count,
        }
    }

    pub fn empty() -> Self {
        Self::new(0, 0)
    }

    pub fn accepts_enqueue(&self) -> bool {
        true
    }
}

/// Command to create one crawl job if no active job exists for the same target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnqueueJob {
    pub feed_url: FeedUrl,
    pub trigger: CrawlJobTrigger,
    pub queue: CrawlJobQueue,
    pub priority: i64,
    pub run_after: DateTime<Utc>,
    pub enqueued_at: DateTime<Utc>,
}

impl EnqueueJob {
    pub fn new(
        feed_url: FeedUrl,
        trigger: CrawlJobTrigger,
        queue: CrawlJobQueue,
        priority: i64,
        run_after: DateTime<Utc>,
        enqueued_at: DateTime<Utc>,
    ) -> Self {
        Self {
            feed_url,
            trigger,
            queue,
            priority,
            run_after,
            enqueued_at,
        }
    }
}

/// Result of trying to enqueue one crawl job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnqueueJobResult {
    Enqueued(CrawlJob),
    AlreadyActive,
}

/// Active job facts attached to one scheduling candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveCrawlJob {
    pub job_id: CrawlJobId,
    pub state: CrawlJobState,
}

impl ActiveCrawlJob {
    pub fn new(job_id: CrawlJobId, state: CrawlJobState) -> Self {
        Self { job_id, state }
    }
}

/// Durable crawl job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlJob {
    pub job_id: CrawlJobId,
    pub feed_url: FeedUrl,
    pub state: CrawlJobState,
    pub trigger: CrawlJobTrigger,
    pub queue: CrawlJobQueue,
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
        queue: CrawlJobQueue,
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

/// Durable crawl job state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlJobState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl CrawlJobState {
    pub const PENDING: &'static str = "pending";
    pub const RUNNING: &'static str = "running";
    pub const SUCCEEDED: &'static str = "succeeded";
    pub const FAILED: &'static str = "failed";
    pub const CANCELLED: &'static str = "cancelled";

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => Self::PENDING,
            Self::Running => Self::RUNNING,
            Self::Succeeded => Self::SUCCEEDED,
            Self::Failed => Self::FAILED,
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
            Self::SUCCEEDED => Ok(Self::Succeeded),
            Self::FAILED => Ok(Self::Failed),
            Self::CANCELLED => Ok(Self::Cancelled),
            value => Err(UnknownCrawlJobValue::new("state", value)),
        }
    }
}

/// Reason a crawl job was enqueued.
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

/// Worker queue lane for one crawl job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrawlJobQueue {
    Default,
    Manual,
    Retry,
}

impl CrawlJobQueue {
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

impl fmt::Display for CrawlJobQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CrawlJobQueue {
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
