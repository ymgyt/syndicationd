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
        use futures_util::StreamExt;
        let mut stream = self
            .http
            .get(url.clone().into_inner())
            .send()
            .await
            .map_err(FetchFeedError::Fetch)?
            .error_for_status()
            .map_err(FetchFeedError::Fetch)?
            .bytes_stream();

        let mut buff = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(FetchFeedError::Fetch)?;
            if buff.len() + chunk.len() > self.buff_limit {
                return Err(FetchFeedError::ResponseLimitExceed);
            }
            buff.extend(chunk);
        }

        self.parse(url, buff.as_slice())
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
}
