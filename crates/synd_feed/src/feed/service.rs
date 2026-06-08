use std::{fmt, sync::Arc, time::Duration};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use feed_rs::parser::{ParseFeedError, Parser};

use crate::types::{Feed, FeedUrl};

pub type FetchFeedResult<T> = std::result::Result<T, FetchFeedError>;
pub type FeedParseResult<T> = std::result::Result<T, FeedParseError>;

/// Compatibility error for callers that only want a parsed feed or failure.
#[derive(Debug, thiserror::Error)]
pub enum FetchFeedError {
    #[error("fetch failed: {0}")]
    Fetch(FeedFetchFailure),
    #[error("body read failed: {0}")]
    BodyRead(FeedFetchFailure),
    #[error("response size limit exceeded")]
    ResponseLimitExceed,
    #[error("unexpected http status: {0}")]
    UnexpectedStatus(FeedHttpStatus),
    #[error("feed was not modified")]
    NotModified,
    #[error("parse failed: {0}")]
    Parse(#[from] FeedParseError),
}

#[async_trait]
pub trait FetchFeed: Send + Sync {
    async fn fetch_feed(&self, request: FeedFetchRequest) -> FeedFetchOutcome;
}

/// Request for fetching one feed URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedFetchRequest {
    pub url: FeedUrl,
    pub conditional: FeedConditionalFetch,
}

impl FeedFetchRequest {
    pub fn new(url: FeedUrl) -> Self {
        Self {
            url,
            conditional: FeedConditionalFetch::default(),
        }
    }

    #[must_use]
    pub fn with_conditional(mut self, conditional: FeedConditionalFetch) -> Self {
        self.conditional = conditional;
        self
    }
}

/// Values used to make a conditional HTTP fetch.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedConditionalFetch {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Low-level HTTP body fetch outcome.
#[derive(Debug, Clone)]
pub enum FeedBodyFetchOutcome {
    Fetched(FeedResponseBody),
    NotModified(FeedHttpResponse),
    BodyReadFailed(FeedBodyReadFailure),
    FetchFailed(FeedFetchFailure),
}

/// High-level feed fetch outcome. Successful bodies are parsed before returning.
#[derive(Debug, Clone)]
pub enum FeedFetchOutcome {
    Fetched(Box<FetchedFeed>),
    NotModified(FeedHttpResponse),
    UnexpectedStatus(FeedResponseBody),
    BodyReadFailed(FeedBodyReadFailure),
    FetchFailed(FeedFetchFailure),
    ParseFailed(FeedParseFailure),
}

/// HTTP response metadata observed while fetching a feed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedHttpResponse {
    pub requested_url: FeedUrl,
    pub response_url: FeedUrl,
    pub status: FeedHttpStatus,
    pub headers: FeedResponseHeaders,
    pub fetched_at: DateTime<Utc>,
}

impl FeedHttpResponse {
    fn new(
        requested_url: FeedUrl,
        response_url: FeedUrl,
        status: FeedHttpStatus,
        headers: FeedResponseHeaders,
        fetched_at: DateTime<Utc>,
    ) -> Self {
        Self {
            requested_url,
            response_url,
            status,
            headers,
            fetched_at,
        }
    }

    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.status.as_u16())
    }

    pub fn is_not_modified(&self) -> bool {
        self.status.as_u16() == 304
    }
}

/// HTTP status code for a feed response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeedHttpStatus(u16);

impl FeedHttpStatus {
    pub fn new(status: u16) -> Self {
        Self(status)
    }

    pub fn as_u16(self) -> u16 {
        self.0
    }
}

impl fmt::Display for FeedHttpStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Selected and raw HTTP response headers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeedResponseHeaders {
    pub raw: Vec<FeedHeader>,
    pub content_type: Option<String>,
    pub content_length: Option<u64>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub retry_after: Option<String>,
}

impl FeedResponseHeaders {
    fn from_header_map(headers: &reqwest::header::HeaderMap) -> Self {
        Self {
            raw: headers
                .iter()
                .map(|(name, value)| {
                    FeedHeader::new(
                        name.as_str().to_owned(),
                        String::from_utf8_lossy(value.as_bytes()).into_owned(),
                    )
                })
                .collect(),
            content_type: header_value(headers, reqwest::header::CONTENT_TYPE),
            content_length: header_value(headers, reqwest::header::CONTENT_LENGTH)
                .and_then(|value| value.parse().ok()),
            etag: header_value(headers, reqwest::header::ETAG),
            last_modified: header_value(headers, reqwest::header::LAST_MODIFIED),
            retry_after: header_value(headers, reqwest::header::RETRY_AFTER),
        }
    }

    pub fn to_json_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.raw).expect("feed response headers serialize")
    }
}

/// One HTTP response header.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FeedHeader {
    pub name: String,
    pub value: String,
}

impl FeedHeader {
    fn new(name: String, value: String) -> Self {
        Self { name, value }
    }
}

/// HTTP response body bytes for a feed fetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedResponseBody {
    pub response: FeedHttpResponse,
    pub bytes: Vec<u8>,
}

impl FeedResponseBody {
    fn new(response: FeedHttpResponse, bytes: Vec<u8>) -> Self {
        Self { response, bytes }
    }
}

/// Body read failure after an HTTP response was observed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedBodyReadFailure {
    pub response: FeedHttpResponse,
    pub failure: FeedFetchFailure,
}

/// Fetch failure classified for callers that need retry/error policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedFetchFailure {
    pub kind: FeedFetchFailureKind,
    pub message: String,
}

impl FeedFetchFailure {
    fn from_reqwest(err: &reqwest::Error) -> Self {
        let kind = if err.is_timeout() {
            FeedFetchFailureKind::Timeout
        } else if err.is_connect() {
            FeedFetchFailureKind::Connect
        } else if err.is_body() {
            FeedFetchFailureKind::Body
        } else if err.is_request() {
            FeedFetchFailureKind::Request
        } else {
            FeedFetchFailureKind::Other
        };

        Self {
            kind,
            message: err.to_string(),
        }
    }

    fn too_large(limit: usize) -> Self {
        Self {
            kind: FeedFetchFailureKind::TooLarge,
            message: format!("response size limit exceeded: {limit} bytes"),
        }
    }
}

impl fmt::Display for FeedFetchFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.as_str(), self.message)
    }
}

impl std::error::Error for FeedFetchFailure {}

/// Coarse fetch failure kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedFetchFailureKind {
    Connect,
    Timeout,
    Request,
    Body,
    TooLarge,
    Unsupported,
    Other,
}

impl FeedFetchFailureKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Connect => "connect",
            Self::Timeout => "timeout",
            Self::Request => "request",
            Self::Body => "body",
            Self::TooLarge => "too_large",
            Self::Unsupported => "unsupported",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for FeedFetchFailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Feed parse failure with the body that failed parse validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedParseFailure {
    pub body: FeedResponseBody,
    pub failure: FeedParseError,
}

impl FeedParseFailure {
    fn new(body: FeedResponseBody, failure: FeedParseError) -> Self {
        Self { body, failure }
    }
}

/// Feed parse error classified for callers that need retry/error policy.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct FeedParseError {
    pub kind: FeedParseErrorKind,
    pub message: String,
}

impl From<ParseFeedError> for FeedParseError {
    fn from(err: ParseFeedError) -> Self {
        match err {
            ParseFeedError::ParseError(kind) => Self {
                kind: FeedParseErrorKind::InvalidFeed,
                message: kind.to_string(),
            },
            ParseFeedError::IoError(err) => Self {
                kind: FeedParseErrorKind::Io,
                message: err.to_string(),
            },
            ParseFeedError::JsonSerde(err) => Self {
                kind: FeedParseErrorKind::JsonFormat,
                message: err.to_string(),
            },
            ParseFeedError::JsonUnsupportedVersion(version) => Self {
                kind: FeedParseErrorKind::JsonUnsupportedVersion,
                message: version,
            },
            ParseFeedError::XmlReader(err) => Self {
                kind: FeedParseErrorKind::XmlFormat,
                message: err.to_string(),
            },
        }
    }
}

/// Coarse feed parse error kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedParseErrorKind {
    InvalidFeed,
    Io,
    JsonFormat,
    JsonUnsupportedVersion,
    XmlFormat,
}

impl FeedParseErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidFeed => "invalid_feed",
            Self::Io => "io",
            Self::JsonFormat => "json_format",
            Self::JsonUnsupportedVersion => "json_unsupported_version",
            Self::XmlFormat => "xml_format",
        }
    }
}

impl fmt::Display for FeedParseErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Parsed feed together with the HTTP body it was parsed from.
#[derive(Debug, Clone)]
pub struct FetchedFeed {
    pub body: FeedResponseBody,
    pub feed: Feed,
}

impl FetchedFeed {
    fn new(body: FeedResponseBody, feed: Feed) -> Self {
        Self { body, feed }
    }

    pub fn url(&self) -> &FeedUrl {
        &self.body.response.requested_url
    }

    pub fn bytes(&self) -> &[u8] {
        &self.body.bytes
    }
}

#[async_trait]
impl<T> FetchFeed for Arc<T>
where
    T: FetchFeed,
{
    async fn fetch_feed(&self, request: FeedFetchRequest) -> FeedFetchOutcome {
        self.as_ref().fetch_feed(request).await
    }
}

/// Feed Process entry point.
#[derive(Clone)]
pub struct FeedService {
    http: reqwest::Client,
    buff_limit: usize,
}

#[async_trait]
impl FetchFeed for FeedService {
    async fn fetch_feed(&self, request: FeedFetchRequest) -> FeedFetchOutcome {
        FeedService::fetch_feed(self, request).await
    }
}

impl FeedService {
    pub fn new(user_agent: &str, buff_limit: usize) -> Self {
        let http = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        Self { http, buff_limit }
    }

    pub async fn fetch_feed(&self, request: FeedFetchRequest) -> FeedFetchOutcome {
        match self.fetch_body(request).await {
            FeedBodyFetchOutcome::Fetched(body) if body.response.is_success() => {
                match self.parse(body.response.requested_url.clone(), body.bytes.as_slice()) {
                    Ok(feed) => FeedFetchOutcome::Fetched(Box::new(FetchedFeed::new(body, feed))),
                    Err(failure) => {
                        FeedFetchOutcome::ParseFailed(FeedParseFailure::new(body, failure))
                    }
                }
            }
            FeedBodyFetchOutcome::Fetched(body) => FeedFetchOutcome::UnexpectedStatus(body),
            FeedBodyFetchOutcome::NotModified(response) => FeedFetchOutcome::NotModified(response),
            FeedBodyFetchOutcome::BodyReadFailed(failure) => {
                FeedFetchOutcome::BodyReadFailed(failure)
            }
            FeedBodyFetchOutcome::FetchFailed(failure) => FeedFetchOutcome::FetchFailed(failure),
        }
    }

    pub async fn fetch_body(&self, request: FeedFetchRequest) -> FeedBodyFetchOutcome {
        use futures_util::StreamExt;

        let mut request_builder = self.http.get(request.url.clone().into_inner());
        if let Some(etag) = &request.conditional.etag {
            request_builder = request_builder.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        if let Some(last_modified) = &request.conditional.last_modified {
            request_builder =
                request_builder.header(reqwest::header::IF_MODIFIED_SINCE, last_modified);
        }

        let response = match request_builder.send().await {
            Ok(response) => response,
            Err(err) => {
                return FeedBodyFetchOutcome::FetchFailed(FeedFetchFailure::from_reqwest(&err));
            }
        };

        let response_meta = FeedHttpResponse::new(
            request.url,
            FeedUrl::from(response.url().clone()),
            FeedHttpStatus::new(response.status().as_u16()),
            FeedResponseHeaders::from_header_map(response.headers()),
            Utc::now(),
        );

        if response_meta.is_not_modified() {
            return FeedBodyFetchOutcome::NotModified(response_meta);
        }

        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = match chunk {
                Ok(chunk) => chunk,
                Err(err) => {
                    return FeedBodyFetchOutcome::BodyReadFailed(FeedBodyReadFailure {
                        response: response_meta,
                        failure: FeedFetchFailure::from_reqwest(&err),
                    });
                }
            };

            if bytes.len() + chunk.len() > self.buff_limit {
                return FeedBodyFetchOutcome::BodyReadFailed(FeedBodyReadFailure {
                    response: response_meta,
                    failure: FeedFetchFailure::too_large(self.buff_limit),
                });
            }

            bytes.extend(chunk);
        }

        FeedBodyFetchOutcome::Fetched(FeedResponseBody::new(response_meta, bytes))
    }

    pub async fn fetch_feed_with_body(&self, url: FeedUrl) -> FetchFeedResult<FetchedFeed> {
        match self.fetch_feed(FeedFetchRequest::new(url)).await {
            FeedFetchOutcome::Fetched(fetched) => Ok(*fetched),
            FeedFetchOutcome::NotModified(_) => Err(FetchFeedError::NotModified),
            FeedFetchOutcome::UnexpectedStatus(body) => {
                Err(FetchFeedError::UnexpectedStatus(body.response.status))
            }
            FeedFetchOutcome::BodyReadFailed(failure) => {
                if failure.failure.kind == FeedFetchFailureKind::TooLarge {
                    Err(FetchFeedError::ResponseLimitExceed)
                } else {
                    Err(FetchFeedError::BodyRead(failure.failure))
                }
            }
            FeedFetchOutcome::FetchFailed(failure) => Err(FetchFeedError::Fetch(failure)),
            FeedFetchOutcome::ParseFailed(failure) => Err(FetchFeedError::Parse(failure.failure)),
        }
    }

    pub async fn read_feed(&self, url: FeedUrl) -> FetchFeedResult<Feed> {
        self.fetch_feed_with_body(url)
            .await
            .map(|fetched| fetched.feed)
    }

    pub fn parse<S>(&self, url: FeedUrl, source: S) -> FeedParseResult<Feed>
    where
        S: std::io::Read,
    {
        Self::parse_feed(url, source)
    }

    /// Parses a feed document without performing an HTTP fetch.
    pub fn parse_feed<S>(url: FeedUrl, source: S) -> FeedParseResult<Feed>
    where
        S: std::io::Read,
    {
        let parser = Self::build_parser(&url);

        parser
            .parse(source)
            .map(|feed| Feed::from_feed_rs(url, feed))
            .map_err(FeedParseError::from)
    }

    fn build_parser(base_uri: impl AsRef<str>) -> Parser {
        feed_rs::parser::Builder::new()
            .base_uri(Some(base_uri))
            .id_generator(|_, _, _| crate::types::feed_rs_missing_id_marker())
            .build()
    }
}

fn header_value(
    headers: &reqwest::header::HeaderMap,
    name: reqwest::header::HeaderName,
) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::assert_matches;
    use feed_rs::parser::ParseErrorKind;

    #[test]
    fn from_feed_rs_parse_feed_error() {
        assert_eq!(
            FeedParseError::from(ParseFeedError::ParseError(ParseErrorKind::NoFeedRoot)).kind,
            FeedParseErrorKind::InvalidFeed
        );
        assert_eq!(
            FeedParseError::from(ParseFeedError::IoError(std::io::Error::from(
                std::io::ErrorKind::UnexpectedEof
            )))
            .kind,
            FeedParseErrorKind::Io
        );
        assert_eq!(
            FeedParseError::from(ParseFeedError::JsonUnsupportedVersion("dummy".into())).kind,
            FeedParseErrorKind::JsonUnsupportedVersion
        );
    }

    #[test]
    fn rss2_entry_updated_uses_item_pub_date() {
        let service = FeedService::new("synd-test", 1024);
        let feed = service
            .parse(
                FeedUrl::parse("https://example.com/rss.xml").unwrap(),
                br#"
                <rss version="2.0">
                    <channel>
                        <title>Example</title>
                        <link>https://example.com/</link>
                        <description>Example feed</description>
                        <lastBuildDate>Mon, 01 Jan 2024 00:00:00 +0000</lastBuildDate>
                        <item>
                            <title>Entry</title>
                            <link>https://example.com/entry</link>
                            <description>Entry body</description>
                            <pubDate>Tue, 02 Jan 2024 03:04:05 +0000</pubDate>
                            <guid>https://example.com/entry</guid>
                        </item>
                    </channel>
                </rss>
                "#
                .as_slice(),
            )
            .unwrap();

        let entry = feed.entries().next().unwrap();
        let feed_updated = "2024-01-01T00:00:00Z".parse().unwrap();
        let entry_published = "2024-01-02T03:04:05Z".parse().unwrap();

        assert_eq!(feed.meta().updated(), Some(feed_updated));
        assert_eq!(entry.published(), Some(entry_published));
        assert_eq!(entry.updated(), Some(entry_published));
        assert_synd_entry_id(entry.id_ref());
        assert!(!entry.id().to_string().contains("https://example.com/entry"));
    }

    #[test]
    fn atom_native_entry_id_is_not_exposed() {
        let service = FeedService::new("synd-test", 1024);
        let feed = service
            .parse(
                FeedUrl::parse("https://example.com/atom.xml").unwrap(),
                br#"
                <feed xmlns="http://www.w3.org/2005/Atom">
                    <title>Example</title>
                    <id>https://example.com/atom.xml</id>
                    <updated>2026-05-24T00:00:00Z</updated>
                    <entry>
                        <title>Native id entry</title>
                        <id>tag:example.com,2026:entry-1</id>
                        <updated>2026-05-24T00:00:00Z</updated>
                    </entry>
                </feed>
                "#
                .as_slice(),
            )
            .unwrap();

        let entry = feed.entries().next().unwrap();
        assert_synd_entry_id(entry.id_ref());
        assert!(!entry.id().to_string().contains("tag:example.com"));
    }

    #[test]
    fn missing_native_entry_id_is_deterministic() {
        let service = FeedService::new("synd-test", 1024);
        let feed_url = FeedUrl::parse("https://example.com/atom.xml").unwrap();
        let source = br#"
            <feed xmlns="http://www.w3.org/2005/Atom">
                <title>Example</title>
                <id>https://example.com/atom.xml</id>
                <updated>2026-05-24T00:00:00Z</updated>
                <entry>
                    <title>Missing id entry</title>
                    <updated>2026-05-24T00:00:00Z</updated>
                </entry>
            </feed>
            "#;

        let first = service.parse(feed_url.clone(), source.as_slice()).unwrap();
        let second = service.parse(feed_url, source.as_slice()).unwrap();

        assert_eq!(
            first.entries().next().unwrap().id(),
            second.entries().next().unwrap().id()
        );
        assert_synd_entry_id(first.entries().next().unwrap().id_ref());
    }

    #[test]
    fn entry_without_entry_id_source_is_skipped() {
        let service = FeedService::new("synd-test", 1024);
        let feed = service
            .parse(
                FeedUrl::parse("https://example.com/atom.xml").unwrap(),
                br#"
                <feed xmlns="http://www.w3.org/2005/Atom">
                    <title>Example</title>
                    <id>https://example.com/atom.xml</id>
                    <updated>2026-05-24T00:00:00Z</updated>
                    <entry>
                        <updated>2026-05-24T00:00:00Z</updated>
                    </entry>
                    <entry>
                        <title>Kept entry</title>
                        <updated>2026-05-24T00:00:00Z</updated>
                    </entry>
                </feed>
                "#
                .as_slice(),
            )
            .unwrap();

        let entries = feed.entries().collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].title(), Some("Kept entry"));
        assert_synd_entry_id(entries[0].id_ref());
    }

    #[test]
    fn parse_invalid_feed_is_typed() {
        let service = FeedService::new("synd-test", 1024);
        let err = service
            .parse(
                FeedUrl::parse("https://example.com/feed.xml").unwrap(),
                b"".as_slice(),
            )
            .unwrap_err();

        assert_matches!(err.kind, FeedParseErrorKind::InvalidFeed);
    }

    fn assert_synd_entry_id(id: &crate::types::EntryId) {
        let id = id.as_str();
        assert!(id.starts_with("synd:entry:v1:"), "{id}");
        assert_eq!(id.len(), "synd:entry:v1:".len() + 64);
        assert!(
            id["synd:entry:v1:".len()..]
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
            "{id}"
        );
    }
}
