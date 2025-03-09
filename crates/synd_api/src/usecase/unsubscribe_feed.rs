use std::sync::Arc;

use synd_feed::types::FeedUrl;
use synd_o11y::metric;

use crate::{
    principal::Principal,
    repository::{self, SubscriptionRepository},
    usecase::{Input, Output},
};

use super::{Usecase, authorize::Unauthorized};

pub struct UnsubscribeFeed {
    pub repository: Arc<dyn SubscriptionRepository>,
}

pub struct UnsubscribeFeedInput {
    pub url: FeedUrl,
}

pub struct UnsubscribeFeedOutput {}

impl Usecase for UnsubscribeFeed {
    type Input = UnsubscribeFeedInput;

    type Output = UnsubscribeFeedOutput;

    type Error = anyhow::Error;

    fn new(make: &super::MakeUsecase) -> Self {
        Self {
            repository: make.subscription_repo.clone(),
        }
    }

    async fn authorize(
        &self,
        principal: Principal,
        _: &UnsubscribeFeedInput,
    ) -> Result<Principal, Unauthorized> {
        Ok(principal)
    }

    async fn usecase(
        &self,
        Input {
            principal,
            input: UnsubscribeFeedInput { url },
            ..
        }: Input<Self::Input>,
    ) -> Result<Output<Self::Output>, super::Error<Self::Error>> {
        tracing::debug!("Unsubscribe feed: {url}");

        self.repository
            .delete_feed_subscription(repository::types::FeedSubscription {
                user_id: principal.user_id().unwrap().to_owned(),
                url,
                requirement: None,
                category: None,
            })
            .await?;

        metric!(monotonic_counter.feed.unsubscription = 1);

        Ok(Output {
            output: UnsubscribeFeedOutput {},
        })
    }
}
