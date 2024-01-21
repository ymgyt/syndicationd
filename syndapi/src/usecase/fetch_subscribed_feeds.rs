use std::sync::Arc;

use synd::{feed::parser::FetchFeed, types};

use crate::{
    persistence::Datastore,
    principal::Principal,
    usecase::{authorize::Unauthorized, Error, Input, MakeUsecase, Output, Usecase},
};

pub struct FetchSubscribedFeeds {
    pub datastore: Arc<dyn Datastore>,
    pub fetch_feed: Arc<dyn FetchFeed>,
}

pub struct FetchSubscribedFeedsInput {
    pub after: Option<String>,
    pub first: usize,
}

#[derive(Default)]
pub struct FetchSubscribedFeedsOutput {
    pub feeds: Vec<types::Feed>,
}

impl Usecase for FetchSubscribedFeeds {
    type Input = FetchSubscribedFeedsInput;

    type Output = FetchSubscribedFeedsOutput;

    type Error = anyhow::Error;

    fn new(make: &MakeUsecase) -> Self {
        Self {
            datastore: make.datastore.clone(),
            fetch_feed: make.fetch_feed.clone(),
        }
    }

    async fn authorize(
        &self,
        principal: Principal,
        _input: &Self::Input,
    ) -> Result<Principal, Unauthorized> {
        Ok(principal)
    }

    async fn usecase(
        &self,
        Input {
            principal,
            input: FetchSubscribedFeedsInput { after, first },
        }: Input<Self::Input>,
    ) -> Result<Output<Self::Output>, Error<Self::Error>> {
        let user_id = principal.user_id().unwrap();

        // fetch all urls from datastore
        let urls = self.datastore.fetch_subscribed_feed_urls(user_id).await?;

        // paginate
        let urls = {
            let start = after
                .and_then(|after| urls.iter().position(|url| url == &after).map(|p| p + 1))
                .unwrap_or(0);
            if start >= urls.len() {
                return Ok(Output {
                    output: FetchSubscribedFeedsOutput::default(),
                });
            }
            let urls = &urls[start..];
            let end = (start + first).min(urls.len());
            &urls[..end]
        };

        // fetch feeds
        // TODO: ignore invalid feed
        let feeds = self
            .fetch_feed
            .fetch_feeds_parallel(urls)
            .await
            .map_err(|err| Error::Usecase(anyhow::Error::from(err)))?;

        Ok(Output {
            output: FetchSubscribedFeedsOutput { feeds },
        })
    }
}
