use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use synd_feed::types::FeedUrl;

#[derive(Debug, Clone)]
pub struct FeedSnapshot {
    pub feed_url: FeedUrl,
    pub body: Vec<u8>,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RefreshState {
    pub feed_url: FeedUrl,
    pub last_attempt_at: Option<DateTime<Utc>>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub last_error_kind: Option<RefreshErrorKind>,
    pub last_error_message: Option<String>,
    pub next_refresh_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefreshErrorKind {
    Fetch,
    ResponseLimitExceeded,
    InvalidFeed,
    Io,
    JsonFormat,
    JsonUnsupportedVersion,
    XmlFormat,
    Other,
}

impl RefreshErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fetch => "fetch",
            Self::ResponseLimitExceeded => "response_limit_exceeded",
            Self::InvalidFeed => "invalid_feed",
            Self::Io => "io",
            Self::JsonFormat => "json_format",
            Self::JsonUnsupportedVersion => "json_unsupported_version",
            Self::XmlFormat => "xml_format",
            Self::Other => "other",
        }
    }
}

impl TryFrom<&str> for RefreshErrorKind {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "fetch" => Ok(Self::Fetch),
            "response_limit_exceeded" => Ok(Self::ResponseLimitExceeded),
            "invalid_feed" => Ok(Self::InvalidFeed),
            "io" => Ok(Self::Io),
            "json_format" => Ok(Self::JsonFormat),
            "json_unsupported_version" => Ok(Self::JsonUnsupportedVersion),
            "xml_format" => Ok(Self::XmlFormat),
            "other" => Ok(Self::Other),
            value => Err(anyhow::anyhow!("unknown refresh error kind: {value}")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefreshStarted {
    pub feed_url: FeedUrl,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct RefreshSuccess {
    pub snapshot: FeedSnapshot,
    pub succeeded_at: DateTime<Utc>,
    pub next_refresh_after: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct RefreshFailure {
    pub feed_url: FeedUrl,
    pub failed_at: DateTime<Utc>,
    pub error_kind: RefreshErrorKind,
    pub error_message: String,
    pub next_refresh_after: Option<DateTime<Utc>>,
}
