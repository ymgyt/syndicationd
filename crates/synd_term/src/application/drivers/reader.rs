use std::sync::Arc;

use futures_util::FutureExt as _;
use synd_support::html::{HtmlConverter, fetch::ArticleFetcher};
use url::Url;

use super::request::{RequestContext, RequestFuture};
use crate::{
    application::{Generation, RequestError, RequestKind},
    event::Event,
};

pub(super) struct ReaderDriver {
    fetcher: ArticleFetcher,
}

impl ReaderDriver {
    pub(super) fn new(converter: Arc<dyn HtmlConverter>) -> Self {
        Self {
            fetcher: ArticleFetcher::new(converter),
        }
    }

    pub(super) fn fetch(
        &self,
        context: RequestContext,
        generation: Generation,
        url: Url,
    ) -> (RequestKind, RequestFuture) {
        let fetcher = self.fetcher.clone();
        (
            RequestKind::FetchArticle { generation },
            async move {
                let page = fetcher
                    .fetch(url)
                    .await
                    .map_err(|error| RequestError::Article(error.into()))?;
                let _ = context.emit(|request_id| Event::ArticleFetched {
                    request_id,
                    generation,
                    article: Arc::new(page.article),
                });
                Ok(())
            }
            .boxed(),
        )
    }
}
