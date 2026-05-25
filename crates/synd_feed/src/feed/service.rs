use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use feed_rs::parser::{ParseErrorKind, ParseFeedError, Parser};

use crate::types::{Feed, FeedUrl};

pub type FetchFeedResult<T> = std::result::Result<T, FetchFeedError>;

#[derive(Debug, thiserror::Error)]
pub enum FetchFeedError {
    #[error("fetch failed")]
    Fetch(#[from] reqwest::Error),
    #[error("response size limit exceeded")]
    ResponseLimitExceed,
    #[error("invalid feed: {0}")]
    InvalidFeed(ParseErrorKind),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json format error: {0}")]
    JsonFormat(#[from] serde_json::Error),
    #[error("unsupported json version: {0}")]
    JsonUnsupportedVersion(String),
    #[error("xml format error: {0}")]
    XmlFormat(String),
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<ParseFeedError> for FetchFeedError {
    fn from(err: ParseFeedError) -> Self {
        match err {
            ParseFeedError::ParseError(kind) => FetchFeedError::InvalidFeed(kind),
            ParseFeedError::IoError(io_err) => FetchFeedError::Io(io_err),
            ParseFeedError::JsonSerde(json_err) => FetchFeedError::JsonFormat(json_err),
            ParseFeedError::JsonUnsupportedVersion(version) => {
                FetchFeedError::JsonUnsupportedVersion(version)
            }
            ParseFeedError::XmlReader(xml_err) => FetchFeedError::XmlFormat(format!("{xml_err}")),
        }
    }
}

#[async_trait]
pub trait FetchFeed: Send + Sync {
    async fn fetch_feed(&self, url: FeedUrl) -> FetchFeedResult<Feed>;
}

#[derive(Debug, Clone)]
pub struct FetchedFeed {
    pub url: FeedUrl,
    pub feed: Feed,
    pub body: Vec<u8>,
    pub fetched_at: chrono::DateTime<chrono::Utc>,
    pub content_type: Option<String>,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

#[async_trait]
impl<T> FetchFeed for Arc<T>
where
    T: FetchFeed,
{
    async fn fetch_feed(&self, url: FeedUrl) -> FetchFeedResult<Feed> {
        self.fetch_feed(url).await
    }
}

/// Feed Process entry point
#[derive(Clone)]
pub struct FeedService {
    http: reqwest::Client,
    buff_limit: usize,
}

#[async_trait]
impl FetchFeed for FeedService {
    async fn fetch_feed(&self, url: FeedUrl) -> FetchFeedResult<Feed> {
        self.fetch_feed_with_body(url)
            .await
            .map(|fetched| fetched.feed)
    }
}

impl FeedService {
    pub async fn fetch_feed_with_body(&self, url: FeedUrl) -> FetchFeedResult<FetchedFeed> {
        use futures_util::StreamExt;
        let response = self
            .http
            .get(url.clone().into_inner())
            .send()
            .await
            .map_err(FetchFeedError::Fetch)?
            .error_for_status()
            .map_err(FetchFeedError::Fetch)?;

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let etag = response
            .headers()
            .get(reqwest::header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);
        let last_modified = response
            .headers()
            .get(reqwest::header::LAST_MODIFIED)
            .and_then(|value| value.to_str().ok())
            .map(ToOwned::to_owned);

        let mut stream = response.bytes_stream();

        let mut buff = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(FetchFeedError::Fetch)?;
            if buff.len() + chunk.len() > self.buff_limit {
                return Err(FetchFeedError::ResponseLimitExceed);
            }
            buff.extend(chunk);
        }

        let feed = self.parse(url.clone(), buff.as_slice())?;

        Ok(FetchedFeed {
            url,
            feed,
            body: buff,
            fetched_at: chrono::Utc::now(),
            content_type,
            etag,
            last_modified,
        })
    }

    pub fn new(user_agent: &str, buff_limit: usize) -> Self {
        let http = reqwest::ClientBuilder::new()
            .user_agent(user_agent)
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap();

        Self { http, buff_limit }
    }

    pub fn parse<S>(&self, url: FeedUrl, source: S) -> FetchFeedResult<Feed>
    where
        S: std::io::Read,
    {
        let parser = Self::build_parser(&url);

        parser
            .parse(source)
            .map(|feed| Feed::from((url, feed)))
            .map_err(FetchFeedError::from)
    }

    fn build_parser(base_uri: impl AsRef<str>) -> Parser {
        feed_rs::parser::Builder::new()
            .base_uri(Some(base_uri))
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_feed_rs_parse_feed_error() {
        assert!(matches!(
            FetchFeedError::from(ParseFeedError::ParseError(ParseErrorKind::NoFeedRoot)),
            FetchFeedError::InvalidFeed(_)
        ));
        assert!(matches!(
            FetchFeedError::from(ParseFeedError::IoError(std::io::Error::from(
                std::io::ErrorKind::UnexpectedEof
            ))),
            FetchFeedError::Io(_)
        ));
        assert!(matches!(
            FetchFeedError::from(ParseFeedError::JsonUnsupportedVersion("dummy".into())),
            FetchFeedError::JsonUnsupportedVersion(_)
        ));
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
    }
}
