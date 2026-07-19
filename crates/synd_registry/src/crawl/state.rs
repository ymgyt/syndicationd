use std::fmt;

use chrono::{DateTime, Utc};
use synd_feed::{
    feed::service::{
        FeedConditionalFetch, FeedFetchFailureKind, FeedHttpStatus, FeedParseErrorKind,
    },
    types::FeedUrl,
};

/// Observation: what crawling has learned about one feed — the summary of
/// the last crawl plus the conditional-fetch context for the next one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlState {
    pub feed_url: FeedUrl,
    pub last: LastCrawlResult,
    pub health: CrawlHealth,
    pub conditional: FeedConditionalFetch,
}

/// Last crawl-result facts projected into current state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastCrawlResult {
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub http_status: Option<FeedHttpStatus>,
    pub error: Option<CrawlStateError>,
    pub retry_after: Option<DateTime<Utc>>,
}

impl LastCrawlResult {
    pub fn normal(
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        http_status: Option<FeedHttpStatus>,
        retry_after: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            started_at,
            finished_at,
            http_status,
            error: None,
            retry_after,
        }
    }

    pub fn abnormal(
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        http_status: Option<FeedHttpStatus>,
        error: CrawlStateError,
        retry_after: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            started_at,
            finished_at,
            http_status,
            error: Some(error),
            retry_after,
        }
    }

    pub fn is_normal(&self) -> bool {
        self.error.is_none()
    }
}

/// Current crawl health facts derived from recent crawl results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrawlHealth {
    pub failure_streak: FailureStreak,
}

impl CrawlHealth {
    pub fn for_last_result(last: &LastCrawlResult, previous: Option<&CrawlState>) -> Self {
        if last.is_normal() {
            Self::healthy()
        } else {
            Self::failed(previous)
        }
    }

    pub fn healthy() -> Self {
        Self {
            failure_streak: FailureStreak::zero(),
        }
    }

    pub fn failed(previous: Option<&CrawlState>) -> Self {
        let previous = previous.map_or(0, |state| state.health.failure_streak.value());
        Self {
            failure_streak: FailureStreak::new(previous.saturating_add(1)),
        }
    }
}

/// Consecutive abnormal crawl-result count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureStreak(u64);

impl FailureStreak {
    pub fn zero() -> Self {
        Self(0)
    }

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn value(self) -> u64 {
        self.0
    }
}

/// Error fact projected into current crawl state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrawlStateError {
    pub kind: CrawlStateErrorKind,
}

impl CrawlStateError {
    pub fn fetch(kind: FeedFetchFailureKind) -> Self {
        Self {
            kind: CrawlStateErrorKind::Fetch(kind),
        }
    }

    pub fn http(kind: CrawlHttpErrorKind) -> Self {
        Self {
            kind: CrawlStateErrorKind::Http(kind),
        }
    }

    pub fn parse(kind: FeedParseErrorKind) -> Self {
        Self {
            kind: CrawlStateErrorKind::Parse(kind),
        }
    }
}

/// Scheduler/query-facing error classification for the latest crawl result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlStateErrorKind {
    Fetch(FeedFetchFailureKind),
    Http(CrawlHttpErrorKind),
    Parse(FeedParseErrorKind),
}

impl fmt::Display for CrawlStateErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fetch(kind) => write!(f, "fetch_{kind}"),
            Self::Http(kind) => write!(f, "http_{kind}"),
            Self::Parse(kind) => write!(f, "parse_{kind}"),
        }
    }
}

/// Registry-specific HTTP status classification for crawl state and scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrawlHttpErrorKind {
    RateLimited,
    Unavailable,
    NotFound,
    Gone,
    ClientError,
    ServerError,
    UnexpectedStatus,
}

impl CrawlHttpErrorKind {
    pub fn from_status(status: FeedHttpStatus) -> Self {
        match status.as_u16() {
            429 => Self::RateLimited,
            503 => Self::Unavailable,
            404 => Self::NotFound,
            410 => Self::Gone,
            400..=499 => Self::ClientError,
            500..=599 => Self::ServerError,
            _ => Self::UnexpectedStatus,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::RateLimited => "rate_limited",
            Self::Unavailable => "unavailable",
            Self::NotFound => "not_found",
            Self::Gone => "gone",
            Self::ClientError => "client_error",
            Self::ServerError => "server_error",
            Self::UnexpectedStatus => "unexpected_status",
        }
    }
}

impl fmt::Display for CrawlHttpErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Command to update current crawl state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertCrawlStateCommand {
    pub feed_url: FeedUrl,
    pub last: LastCrawlResult,
    pub health: CrawlHealth,
    pub conditional: FeedConditionalFetch,
}

impl UpsertCrawlStateCommand {
    pub fn new(
        feed_url: FeedUrl,
        last: LastCrawlResult,
        health: CrawlHealth,
        conditional: FeedConditionalFetch,
    ) -> Self {
        Self {
            feed_url,
            last,
            health,
            conditional,
        }
    }
}
