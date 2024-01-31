use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use feed_rs::parser::Parser;

use crate::types::Feed;

pub type ParseResult<T> = std::result::Result<T, ParserError>;

#[derive(Debug, thiserror::Error)]
pub enum ParserError {
    #[error("fetch failed")]
    Fetch(#[from] reqwest::Error),
    #[error("response size limit exceeded")]
    ResponseLimitExceed,

    #[error("parse error url: {url} {source}")]
    Parse { url: String, source: anyhow::Error },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

#[async_trait]
pub trait FetchFeed: Send + Sync {
    async fn fetch_feed(&self, url: String) -> ParseResult<Feed>;
    /// Fetch feeds by spawing tasks
    async fn fetch_feeds_parallel(&self, urls: &[String]) -> ParseResult<Vec<Feed>>;
}

#[async_trait]
impl<T> FetchFeed for Arc<T>
where
    T: FetchFeed,
{
    async fn fetch_feed(&self, url: String) -> ParseResult<Feed> {
        self.fetch_feed(url).await
    }
    /// Fetch feeds by spawing tasks
    async fn fetch_feeds_parallel(&self, urls: &[String]) -> ParseResult<Vec<Feed>> {
        self.fetch_feeds_parallel(urls).await
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
    async fn fetch_feed(&self, url: String) -> ParseResult<Feed> {
        use futures_util::StreamExt;
        let mut stream = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(ParserError::Fetch)?
            .error_for_status()
            .map_err(ParserError::Fetch)?
            .bytes_stream();

        let mut buff = Vec::new();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(ParserError::Fetch)?;
            if buff.len() + chunk.len() > self.buff_limit {
                return Err(ParserError::ResponseLimitExceed);
            }
            buff.extend(chunk);
        }

        self.parse(url, buff.as_slice())
    }

    async fn fetch_feeds_parallel(&self, urls: &[String]) -> ParseResult<Vec<Feed>> {
        // Order is matter, so we could not use tokio JoinSet or futures FuturesUnordered
        // should use FuturesOrders ?
        let mut handles = Vec::with_capacity(urls.len());
        for url in urls {
            let this = self.clone();
            let url = url.clone();
            handles.push(tokio::task::spawn(
                async move { this.fetch_feed(url).await },
            ));
        }

        let mut feeds = Vec::with_capacity(handles.len());
        for handle in handles {
            feeds.push(handle.await.expect("tokio spawn join error")?);
        }

        Ok(feeds)
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

    pub fn parse<S>(&self, url: impl Into<String>, source: S) -> ParseResult<Feed>
    where
        S: std::io::Read,
    {
        let url = url.into();
        let parser = self.build_parser(&url);

        match parser.parse(source) {
            Ok(feed) => Ok(Feed::from((url, feed))),
            // TODO: handle error
            Err(err) => Err(ParserError::Parse {
                url,
                source: anyhow::Error::from(err),
            }),
        }
    }

    fn build_parser(&self, base_uri: impl AsRef<str>) -> Parser {
        feed_rs::parser::Builder::new()
            .base_uri(Some(base_uri))
            .build()
    }
}
