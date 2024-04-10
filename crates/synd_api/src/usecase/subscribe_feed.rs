use std::sync::Arc;

use synd_feed::{
    feed::{cache::FetchCachedFeed, parser::FetchFeedError},
    types::{Annotated, Category, Feed, Requirement},
};
use synd_o11y::metric;
use thiserror::Error;

use crate::{
    principal::Principal,
    repository::{self, SubscriptionRepository},
    usecase::{Input, Output},
};

use super::{authorize::Unauthorized, Usecase};

pub struct SubscribeFeed {
    pub repository: Arc<dyn SubscriptionRepository>,
    pub fetch_feed: Arc<dyn FetchCachedFeed>,
}

pub struct SubscribeFeedInput {
    pub url: String,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}

pub struct SubscribeFeedOutput {
    pub feed: Annotated<Arc<Feed>>,
}

#[derive(Error, Debug)]
pub enum SubscribeFeedError {
    #[error("fetch feed error: {0}")]
    FetchFeed(FetchFeedError),
}

impl Usecase for SubscribeFeed {
    type Input = SubscribeFeedInput;

    type Output = SubscribeFeedOutput;

    type Error = SubscribeFeedError;

    fn new(make: &super::MakeUsecase) -> Self {
        Self {
            repository: make.subscription_repo.clone(),
            fetch_feed: make.fetch_feed.clone(),
        }
    }

    async fn authorize(
        &self,
        principal: Principal,
        _: &SubscribeFeedInput,
    ) -> Result<Principal, Unauthorized> {
        Ok(principal)
    }

    async fn usecase(
        &self,
        Input {
            principal,
            input:
                SubscribeFeedInput {
                    url,
                    requirement,
                    category,
                },
            ..
        }: Input<Self::Input>,
    ) -> Result<Output<Self::Output>, super::Error<Self::Error>> {
        tracing::debug!("Subscribe feed: {url}");

        let feed = self
            .fetch_feed
            .fetch_feed(url.clone())
            .await
            .map_err(|err| super::Error::Usecase(SubscribeFeedError::FetchFeed(err)))?;

        tracing::debug!("{:?}", feed.meta());

        self.repository
            .put_feed_subscription(repository::types::FeedSubscription {
                user_id: principal.user_id().unwrap().to_owned(),
                url: feed.meta().url().to_owned(),
                requirement,
                category: category.clone(),
            })
            .await?;

        metric!(monotonic_counter.feed.subscription = 1);

        let feed = Annotated {
            feed,
            requirement,
            category,
        };

        Ok(Output {
            output: SubscribeFeedOutput { feed },
        })
    }
}
