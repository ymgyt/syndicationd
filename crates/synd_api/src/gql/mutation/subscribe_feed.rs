use async_graphql::{InputObject, Object, Union};
use synd_feed::{
    feed::service::FetchFeedError,
    types::{Category, FeedUrl, Requirement},
};

use crate::{
    gql::{
        mutation::ResponseStatus,
        object::{self, Feed},
    },
    usecase::{self, SubscribeFeedError as UsecaseSubscribeFeedError},
};

#[derive(InputObject, Debug)]
pub(crate) struct SubscribeFeedInput {
    /// Feed url to subscribe
    pub url: FeedUrl,
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
pub(crate) enum SubscribeFeedResponse {
    Success(SubscribeFeedSuccess),
    Error(SubscribeFeedError),
}

pub(crate) struct SubscribeFeedSuccess {
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

pub(crate) struct SubscribeFeedError {
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
                FetchFeedError::Fetch(request_err) => Self {
                    status: ResponseStatus::feed_unavailable(),
                    message: format!("feed unavailable: {request_err}"),
                },
                fetch_err => Self {
                    status: ResponseStatus::internal(),
                    message: format!("{fetch_err}"),
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_error() {
        let r = SubscribeFeedError::from(UsecaseSubscribeFeedError::FetchFeed(
            FetchFeedError::Other(anyhow::anyhow!("error")),
        ));

        assert_eq!(r.status, ResponseStatus::internal());
    }
}
