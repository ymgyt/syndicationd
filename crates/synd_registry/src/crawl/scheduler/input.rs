use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;

use crate::crawl::result::FailureStreak;

/// Scheduler-facing crawl fact submitted before dispatch ordering is decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SchedInput {
    ScheduledDue(ScheduledDue),
    ManualRequested(ManualRequested),
    RetryDue(RetryDue),
    CrawlFinished(CrawlFinished),
}

/// Periodic crawl due fact reconstructed from durable crawl schedule state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ScheduledDue {
    pub(crate) feed_url: FeedUrl,
    pub(crate) due_at: DateTime<Utc>,
}

impl ScheduledDue {
    pub(crate) fn new(feed_url: FeedUrl, due_at: DateTime<Utc>) -> Self {
        Self { feed_url, due_at }
    }
}

impl From<ScheduledDue> for SchedInput {
    fn from(value: ScheduledDue) -> Self {
        Self::ScheduledDue(value)
    }
}

/// Manual crawl request submitted by API/UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManualRequested {
    pub(crate) feed_url: FeedUrl,
    pub(crate) requested_at: DateTime<Utc>,
}

impl ManualRequested {
    pub(crate) fn new(feed_url: FeedUrl, requested_at: DateTime<Utc>) -> Self {
        Self {
            feed_url,
            requested_at,
        }
    }
}

impl From<ManualRequested> for SchedInput {
    fn from(value: ManualRequested) -> Self {
        Self::ManualRequested(value)
    }
}

/// Retry crawl due fact reconstructed from durable crawl state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RetryDue {
    pub(crate) feed_url: FeedUrl,
    pub(crate) due_at: DateTime<Utc>,
    pub(crate) failure_streak: FailureStreak,
}

impl RetryDue {
    pub(crate) fn new(
        feed_url: FeedUrl,
        due_at: DateTime<Utc>,
        failure_streak: FailureStreak,
    ) -> Self {
        Self {
            feed_url,
            due_at,
            failure_streak,
        }
    }
}

impl From<RetryDue> for SchedInput {
    fn from(value: RetryDue) -> Self {
        Self::RetryDue(value)
    }
}

/// Crawl completion signal used to release scheduler-local inflight state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CrawlFinished {
    pub(crate) feed_url: FeedUrl,
    pub(crate) finished_at: DateTime<Utc>,
}

impl CrawlFinished {
    pub(crate) fn new(feed_url: FeedUrl, finished_at: DateTime<Utc>) -> Self {
        Self {
            feed_url,
            finished_at,
        }
    }
}

impl From<CrawlFinished> for SchedInput {
    fn from(value: CrawlFinished) -> Self {
        Self::CrawlFinished(value)
    }
}
