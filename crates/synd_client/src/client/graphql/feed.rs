use synd_feed::types::FeedUrl;
use tracing::instrument;

use super::GraphqlRequest;
use crate::{
    Client, SyndApiError,
    payload::{
        SubscribeFeedInput, SubscribeFeedPayload, SubscriptionPayload, UnsubscribeFeedPayload,
    },
};

const FETCH_SUBSCRIPTION_QUERY: &str = include_str!("query/fetch_subscription.gql");
const SUBSCRIBE_FEED_MUTATION: &str = include_str!("query/subscribe_feed.gql");
const UNSUBSCRIBE_FEED_MUTATION: &str = include_str!("query/unsubscribe_feed.gql");

#[derive(Debug, serde::Serialize)]
struct FetchSubscriptionVariables {
    after: Option<String>,
    first: Option<i64>,
}

#[derive(Debug, serde::Deserialize)]
struct FetchSubscriptionData {
    output: SubscriptionPayload,
}

impl From<FetchSubscriptionData> for SubscriptionPayload {
    fn from(data: FetchSubscriptionData) -> Self {
        data.output
    }
}

#[derive(Debug, serde::Serialize)]
struct SubscribeFeedVariables {
    input: SubscribeFeedInput,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubscribeFeedData {
    subscribe_feed: SubscribeFeedPayload,
}

impl From<SubscribeFeedData> for SubscribeFeedPayload {
    fn from(data: SubscribeFeedData) -> Self {
        data.subscribe_feed
    }
}

#[derive(Debug, serde::Serialize)]
struct UnsubscribeFeedVariables {
    input: UnsubscribeFeedInput,
}

#[derive(Debug, serde::Serialize)]
struct UnsubscribeFeedInput {
    url: FeedUrl,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnsubscribeFeedData {
    unsubscribe_feed: UnsubscribeFeedPayload,
}

impl From<UnsubscribeFeedData> for UnsubscribeFeedPayload {
    fn from(data: UnsubscribeFeedData) -> Self {
        data.unsubscribe_feed
    }
}

impl Client {
    #[instrument(skip(self))]
    pub async fn fetch_subscription(
        &self,
        after: Option<String>,
        first: Option<i64>,
    ) -> Result<SubscriptionPayload, SyndApiError> {
        let data: FetchSubscriptionData = self
            .execute_graphql(&GraphqlRequest::new(
                FETCH_SUBSCRIPTION_QUERY,
                FetchSubscriptionVariables { after, first },
            ))
            .await?
            .require_complete()?;
        Ok(data.into())
    }

    #[instrument(skip(self))]
    pub async fn subscribe_feed(
        &self,
        input: SubscribeFeedInput,
    ) -> Result<SubscribeFeedPayload, SyndApiError> {
        let data: SubscribeFeedData = self
            .execute_graphql(&GraphqlRequest::new(
                SUBSCRIBE_FEED_MUTATION,
                SubscribeFeedVariables { input },
            ))
            .await?
            .require_complete()?;
        Ok(data.into())
    }

    #[instrument(skip(self))]
    pub async fn unsubscribe_feed(
        &self,
        url: FeedUrl,
    ) -> Result<UnsubscribeFeedPayload, SyndApiError> {
        let data: UnsubscribeFeedData = self
            .execute_graphql(&GraphqlRequest::new(
                UNSUBSCRIBE_FEED_MUTATION,
                UnsubscribeFeedVariables {
                    input: UnsubscribeFeedInput { url },
                },
            ))
            .await?
            .require_complete()?;
        Ok(data.into())
    }
}
