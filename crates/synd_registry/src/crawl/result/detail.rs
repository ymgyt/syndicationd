use chrono::{DateTime, Utc};
use synd_feed::{
    feed::service::{FeedFetchFailureKind, FeedHttpStatus, FeedParseErrorKind},
    types::FeedUrl,
};

use crate::crawl::blob::BlobRef;

/// Persisted detail shape for one crawl result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrawlResultDetail {
    Fetched {
        http: CrawlHttpResponseDetail,
        body: CrawlHttpBodyDetail,
    },
    NotModified {
        http: CrawlHttpResponseDetail,
    },
    UnexpectedStatus {
        http: CrawlHttpResponseDetail,
        body: CrawlHttpBodyDetail,
    },
    BodyReadFailed {
        http: CrawlHttpResponseDetail,
        error: CrawlFetchErrorDetail,
    },
    FetchFailed {
        error: CrawlFetchErrorDetail,
    },
    ParseFailed {
        http: CrawlHttpResponseDetail,
        body: CrawlHttpBodyDetail,
        error: CrawlFeedParseErrorDetail,
    },
}

impl CrawlResultDetail {
    pub fn http_response(&self) -> Option<&CrawlHttpResponseDetail> {
        match self {
            Self::Fetched { http, .. }
            | Self::NotModified { http }
            | Self::UnexpectedStatus { http, .. }
            | Self::BodyReadFailed { http, .. }
            | Self::ParseFailed { http, .. } => Some(http),
            Self::FetchFailed { .. } => None,
        }
    }

    pub fn body(&self) -> Option<&CrawlHttpBodyDetail> {
        match self {
            Self::Fetched { body, .. }
            | Self::UnexpectedStatus { body, .. }
            | Self::ParseFailed { body, .. } => Some(body),
            Self::NotModified { .. } | Self::BodyReadFailed { .. } | Self::FetchFailed { .. } => {
                None
            }
        }
    }

    pub fn fetch_error(&self) -> Option<&CrawlFetchErrorDetail> {
        match self {
            Self::BodyReadFailed { error, .. } | Self::FetchFailed { error } => Some(error),
            Self::Fetched { .. }
            | Self::NotModified { .. }
            | Self::UnexpectedStatus { .. }
            | Self::ParseFailed { .. } => None,
        }
    }

    pub fn feed_parse_error(&self) -> Option<&CrawlFeedParseErrorDetail> {
        match self {
            Self::ParseFailed { error, .. } => Some(error),
            Self::Fetched { .. }
            | Self::NotModified { .. }
            | Self::UnexpectedStatus { .. }
            | Self::BodyReadFailed { .. }
            | Self::FetchFailed { .. } => None,
        }
    }
}

/// HTTP response detail for one crawl result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlHttpResponseDetail {
    pub status: FeedHttpStatus,
    pub response_url: FeedUrl,
    pub headers_blob: BlobRef,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub retry_after_at: Option<DateTime<Utc>>,
}

/// Stored HTTP response body for result shapes that observed a complete body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CrawlHttpBodyDetail {
    pub body_blob: BlobRef,
}

impl CrawlHttpBodyDetail {
    pub fn new(body_blob: BlobRef) -> Self {
        Self { body_blob }
    }
}

/// Fetch error detail for one crawl result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlFetchErrorDetail {
    pub kind: FeedFetchFailureKind,
    pub message: String,
}

/// Feed parse error detail for one crawl result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrawlFeedParseErrorDetail {
    pub kind: FeedParseErrorKind,
    pub message: String,
}
