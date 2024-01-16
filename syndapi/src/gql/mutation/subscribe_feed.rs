use async_graphql::{InputObject, Object, Union};

use crate::gql::{mutation::ResponseStatus, object::FeedMeta};

#[derive(InputObject)]
pub struct SubscribeFeedInput {
    pub url: String,
}

#[derive(Union)]
pub enum SubscribeFeedResponse {
    Success(SubscribeFeedSuccess),
    Error(SubscribeFeedError),
}

pub struct SubscribeFeedSuccess {
    pub status: ResponseStatus,
    /// Subscribed url
    pub feed: FeedMeta,
}

#[Object]
impl SubscribeFeedSuccess {
    pub async fn status(&self) -> ResponseStatus {
        self.status.clone()
    }

    pub async fn feed(&self) -> &FeedMeta {
        &self.feed
    }
}

pub struct SubscribeFeedError {
    pub status: ResponseStatus,
    pub message: String,
}

#[Object]
impl SubscribeFeedError {
    pub async fn status(&self) -> ResponseStatus {
        self.status.clone()
    }

    /// Error message
    pub async fn message(&self) -> String {
        self.message.clone()
    }
}

impl From<ResponseStatus> for SubscribeFeedResponse {
    fn from(status: ResponseStatus) -> Self {
        SubscribeFeedResponse::Error(SubscribeFeedError {
            status,
            message: "Unauthorized".into(),
        })
    }
}
