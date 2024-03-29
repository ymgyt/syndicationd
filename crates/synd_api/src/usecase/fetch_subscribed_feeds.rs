use std::sync::Arc;

use synd_feed::{feed::cache::FetchCachedFeed, types};
use thiserror::Error;

use crate::{
    principal::Principal,
    repository::SubscriptionRepository,
    usecase::{authorize::Unauthorized, Error, Input, MakeUsecase, Output, Usecase},
};

pub struct FetchSubscribedFeeds {
    pub repository: Arc<dyn SubscriptionRepository>,
    pub fetch_feed: Arc<dyn FetchCachedFeed>,
}

pub struct FetchSubscribedFeedsInput {
    pub after: Option<String>,
    pub first: usize,
}

#[derive(Default)]
pub struct FetchSubscribedFeedsOutput {
    pub feeds: Vec<Arc<types::Feed>>,
}

#[derive(Error, Debug)]
pub enum FetchSubscribedFeedsError {}

impl Usecase for FetchSubscribedFeeds {
    type Input = FetchSubscribedFeedsInput;

    type Output = FetchSubscribedFeedsOutput;

    type Error = FetchSubscribedFeedsError;

    fn new(make: &MakeUsecase) -> Self {
        Self {
            repository: make.subscription_repo.clone(),
            fetch_feed: make.fetch_feed.clone(),
        }
    }

    async fn authorize(
        &self,
        principal: Principal,
        _: &Self::Input,
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

        // fetch all urls from repository
        let urls = self.repository.fetch_subscribed_feed_urls(user_id).await?;

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
        let feeds = self.fetch_feed.fetch_feeds_parallel(urls).await;

        // TODO: return failed feeds
        let (feeds, errors): (Vec<_>, Vec<_>) = feeds.into_iter().partition(Result::is_ok);

        if !errors.is_empty() {
            tracing::error!("{errors:?}");
        }

        let feeds = feeds.into_iter().map(Result::unwrap).collect();

        Ok(Output {
            output: FetchSubscribedFeedsOutput { feeds },
        })
    }
}
