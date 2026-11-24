use std::{sync::Arc, time::Duration};

use anyhow::{Context as _, Result, ensure};
use futures_util::FutureExt as _;
use reqwest::{Client, header};
use synd_support::html::{Article, ArticleRequest, HtmlConverter};
use url::Url;

use super::request::{RequestContext, RequestFuture};
use crate::{
    application::{Generation, RequestError, RequestKind},
    event::Event,
};

const MAX_HTML_BYTES: usize = 8 * 1024 * 1024;

pub(super) struct ReaderDriver {
    http: Client,
    converter: Arc<dyn HtmlConverter>,
}

impl ReaderDriver {
    pub(super) fn new(converter: Arc<dyn HtmlConverter>) -> Self {
        Self {
            http: Client::new(),
            converter,
        }
    }

    pub(super) fn fetch(
        &self,
        context: RequestContext,
        generation: Generation,
        url: Url,
    ) -> (RequestKind, RequestFuture) {
        let http = self.http.clone();
        let converter = Arc::clone(&self.converter);
        (
            RequestKind::FetchArticle { generation },
            async move {
                let article = fetch_article(&http, url, converter)
                    .await
                    .map_err(RequestError::Article)?;
                context.emit(|request_id| Event::ArticleFetched {
                    request_id,
                    generation,
                    article: Arc::new(article),
                });
                Ok(())
            }
            .boxed(),
        )
    }
}

async fn fetch_article(
    http: &Client,
    url: Url,
    converter: Arc<dyn HtmlConverter>,
) -> Result<Article> {
    let mut response = http
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
        .and_then(|value| value.parse::<mime::Mime>().ok());
    if let Some(media) = &media_type {
        ensure!(
            matches!(media.essence_str(), "text/html" | "application/xhtml+xml"),
            "expected an HTML article, received {media}"
        );
    }
    let encoding = media_type
        .as_ref()
        .and_then(|media| media.get_param(mime::CHARSET))
        .and_then(|charset| encoding_rs::Encoding::for_label(charset.as_str().as_bytes()))
        .unwrap_or(encoding_rs::UTF_8);
    let final_url = response.url().clone();
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            chunk.len() <= MAX_HTML_BYTES - bytes.len(),
            "article exceeds 8 MiB"
        );
        bytes.extend_from_slice(&chunk);
    }
    tokio::task::spawn_blocking(move || {
        let (html, _, _) = encoding.decode(&bytes);
        converter.extract_article(
            ArticleRequest::builder()
                .html(&html)
                .base_url(&final_url)
                .build(),
        )
    })
    .await
    .context("article extraction task failed")?
    .map_err(Into::into)
}
