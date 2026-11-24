//! Fetch publisher pages and extract their articles.

use std::{sync::Arc, time::Duration};

use encoding_rs::{Encoding, UTF_8};
use mime::Mime;
use reqwest::{Client, header};
use thiserror::Error;
use tokio::task;
use url::Url;

use super::{Article, ArticleRequest, HtmlConvError, HtmlConverter};

const MAX_HTML_BYTES: usize = 8 * 1024 * 1024;

/// Fetches HTML pages and resolves article links against the final response URL.
#[derive(Clone)]
pub struct ArticleFetcher {
    http: Client,
    converter: Arc<dyn HtmlConverter>,
}

/// Extracted article and the URL that served its HTML after redirects.
#[derive(Debug)]
pub struct FetchedArticle {
    pub url: Url,
    pub article: Article,
}

#[derive(Debug, Error)]
pub enum ArticleFetchError {
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error("expected an HTML article, received {0}")]
    UnsupportedMediaType(Mime),
    #[error("article exceeds 8 MiB")]
    TooLarge,
    #[error("article extraction task failed")]
    ExtractionTask(#[from] task::JoinError),
    #[error(transparent)]
    Conversion(#[from] HtmlConvError),
}

impl ArticleFetcher {
    pub fn new(converter: Arc<dyn HtmlConverter>) -> Self {
        Self {
            http: Client::new(),
            converter,
        }
    }

    pub async fn fetch(&self, url: Url) -> Result<FetchedArticle, ArticleFetchError> {
        let mut response = self
            .http
            .get(url)
            .header(
                header::USER_AGENT,
                concat!("synd/", env!("CARGO_PKG_VERSION")),
            )
            .header(header::ACCEPT, "text/html, application/xhtml+xml")
            .timeout(Duration::from_secs(20))
            .send()
            .await?
            .error_for_status()?;
        let media_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());
        if let Some(media) = &media_type
            && !matches!(media.essence_str(), "text/html" | "application/xhtml+xml")
        {
            return Err(ArticleFetchError::UnsupportedMediaType(media.clone()));
        }
        let encoding = media_type
            .as_ref()
            .and_then(|media| media.get_param(mime::CHARSET))
            .and_then(|charset| Encoding::for_label(charset.as_str().as_bytes()))
            .unwrap_or(UTF_8);
        let url = response.url().clone();
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if chunk.len() > MAX_HTML_BYTES - bytes.len() {
                return Err(ArticleFetchError::TooLarge);
            }
            bytes.extend_from_slice(&chunk);
        }
        let converter = Arc::clone(&self.converter);
        task::spawn_blocking(move || {
            let (html, _, _) = encoding.decode(&bytes);
            let article = converter
                .extract_article(ArticleRequest::builder().html(&html).base_url(&url).build())?;
            Ok(FetchedArticle { url, article })
        })
        .await?
    }
}
