use std::sync::Arc;

use synd::{feed::parser::FetchFeed, types::FeedMeta};

use crate::{
    audit,
    persistence::Datastore,
    principal::Principal,
    serve::layer::audit::Audit,
    usecase::{Input, Output},
};

use super::{authorize::Unauthorized, Usecase};

pub struct SubscribeFeed {
    pub datastore: Arc<dyn Datastore>,
    pub fetch_feed: Arc<dyn FetchFeed>,
}

pub struct SubscribeFeedInput {
    pub url: String,
}

pub struct SubscribeFeedOutput {
    pub feed: FeedMeta,
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
        let feed = self
            .fetch_feed
            .fetch(url.clone())
            .await
            .map_err(|err| super::Error::Usecase(anyhow::Error::from(err)))?;

        self.datastore
            .add_feed_to_subscription(principal.user_id().unwrap(), feed.title().to_owned(), url)
            .await?;

        audit!(
            { Audit::USER_ID } = principal.user_id().unwrap(),
            { Audit::OPERATION } = "subscribe_feed",
            { Audit::RESULT } = "success",
        );

        Ok(Output {
            output: SubscribeFeedOutput { feed: feed.meta() },
        })
    }
}
