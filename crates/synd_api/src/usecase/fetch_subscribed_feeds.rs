use std::sync::Arc;

use synd_feed::{
    feed::{cache::FetchCachedFeed, service::FetchFeedError},
    types::{self, Annotated, FeedUrl},
};
use thiserror::Error;

use crate::{
    principal::Principal,
    repository::{SubscriptionRepository, types::SubscribedFeeds},
    usecase::{Error, Input, MakeUsecase, Output, Usecase, authorize::Unauthorized},
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
    #[allow(clippy::type_complexity)]
    pub feeds: Vec<Result<Annotated<Arc<types::Feed>>, (FeedUrl, FetchFeedError)>>,
}

#[derive(Error, Debug)]
#[error("fetch subscribed feeds error")]
pub struct FetchSubscribedFeedsError {}

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

        let SubscribedFeeds {
            mut urls,
            mut annotations,
        } = self.repository.fetch_subscribed_feeds(user_id).await?;

        // paginate
        let urls = {
            let start = after
                .and_then(|after| {
                    urls.iter()
                        .position(|url| url.as_str() == after)
                        .map(|p| p + 1)
                })
                .unwrap_or(0);
            if start >= urls.len() {
                return Ok(Output {
                    output: FetchSubscribedFeedsOutput::default(),
                });
            }
            let mut urls = urls.split_off(start);
            urls.truncate(first);
            urls
        };

        // fetch feeds
        let fetched_feeds = self.fetch_feed.fetch_feeds_parallel(&urls).await;

        // annotate fetched feeds
        let feeds = fetched_feeds
            .into_iter()
            .zip(urls)
            .map(|(result, url)| {
                result
                    .map(|feed| {
                        match annotations
                            .as_mut()
                            .and_then(|annotations| annotations.remove(feed.meta().url()))
                        {
                            Some(annotations) => Annotated {
                                feed,
                                requirement: annotations.requirement,
                                category: annotations.category,
                            },
                            None => Annotated {
                                feed,
                                requirement: None,
                                category: None,
                            },
                        }
                    })
                    .map_err(|err| (url.clone(), err))
            })
            .collect::<Vec<_>>();

        Ok(Output {
            output: FetchSubscribedFeedsOutput { feeds },
        })
    }
}
