use async_graphql::{InputObject, Object, Union};
use synd_feed::{
    feed::parser::FetchFeedError,
    types::{Category, Requirement},
};

use crate::{
    gql::{
        mutation::ResponseStatus,
        object::{self, Feed},
    },
    usecase::{self, SubscribeFeedError as UsecaseSubscribeFeedError},
};

#[derive(InputObject, Debug)]
pub struct SubscribeFeedInput {
    /// Feed url to subscribe
    pub url: String,
    /// Requirement level for feed
    pub requirement: Option<Requirement>,
    /// Feed category
    pub category: Option<Category<'static>>,
}

impl From<SubscribeFeedInput> for usecase::SubscribeFeedInput {
    fn from(value: SubscribeFeedInput) -> Self {
        usecase::SubscribeFeedInput {
            url: value.url,
            requirement: value.requirement,
            category: value.category,
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Union)]
pub enum SubscribeFeedResponse {
    Success(SubscribeFeedSuccess),
    Error(SubscribeFeedError),
}

pub struct SubscribeFeedSuccess {
    pub status: ResponseStatus,
    /// Subscribed feed
    pub feed: object::Feed,
}

#[Object]
impl SubscribeFeedSuccess {
    pub async fn status(&self) -> ResponseStatus {
        self.status.clone()
    }

    pub async fn feed(&self) -> &object::Feed {
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

impl From<usecase::Output<usecase::SubscribeFeedOutput>> for SubscribeFeedResponse {
    fn from(output: usecase::Output<usecase::SubscribeFeedOutput>) -> Self {
        SubscribeFeedResponse::Success(SubscribeFeedSuccess {
            status: ResponseStatus::ok(),
            feed: Feed::from(output.output.feed),
        })
    }
}

impl From<UsecaseSubscribeFeedError> for SubscribeFeedResponse {
    fn from(err: UsecaseSubscribeFeedError) -> Self {
        SubscribeFeedResponse::Error(err.into())
    }
}

impl From<UsecaseSubscribeFeedError> for SubscribeFeedError {
    fn from(err: UsecaseSubscribeFeedError) -> Self {
        match err {
            UsecaseSubscribeFeedError::FetchFeed(fetch_err) => match fetch_err {
                FetchFeedError::InvalidFeed(kind) => Self {
                    status: ResponseStatus::invalid_feed_url(),
                    message: format!("{kind}"),
                },
                fetch_err => Self {
                    status: ResponseStatus::internal(),
                    message: format!("{fetch_err}"),
                },
            },
        }
    }
}
