use std::sync::Arc;

use synd::{feed::cache::FetchCachedFeed, types::Feed};

use crate::{
    persistence::{self, Datastore},
    principal::Principal,
    usecase::{Input, Output},
};

use super::{authorize::Unauthorized, Usecase};

pub struct SubscribeFeed {
    pub datastore: Arc<dyn Datastore>,
    pub fetch_feed: Arc<dyn FetchCachedFeed>,
}

pub struct SubscribeFeedInput {
    pub url: String,
}

pub struct SubscribeFeedOutput {
    pub feed: Arc<Feed>,
}

impl Usecase for SubscribeFeed {
    type Input = SubscribeFeedInput;

    type Output = SubscribeFeedOutput;

    type Error = anyhow::Error;

    fn new(make: &super::MakeUsecase) -> Self {
        Self {
            datastore: make.datastore.clone(),
            fetch_feed: make.fetch_feed.clone(),
        }
    }

    async fn authorize(
        &self,
        principal: Principal,
        _input: &SubscribeFeedInput,
    ) -> Result<Principal, Unauthorized> {
        Ok(principal)
    }

    async fn usecase(
        &self,
        Input {
            principal,
            input: SubscribeFeedInput { url },
            ..
        }: Input<Self::Input>,
    ) -> Result<Output<Self::Output>, super::Error<Self::Error>> {
        tracing::debug!("Subscribe feed: {url}");

        let feed = self
            .fetch_feed
            .fetch_feed(url.clone())
            .await
            .map_err(|err| super::Error::Usecase(anyhow::Error::from(err)))?;

        tracing::debug!("{:#?}", feed.meta());

        self.datastore
            .put_feed_subscription(persistence::types::FeedSubscription {
                user_id: principal.user_id().unwrap().to_owned(),
                url: feed.meta().url().to_owned(),
            })
            .await?;

        Ok(Output {
            output: SubscribeFeedOutput { feed },
        })
    }
}
