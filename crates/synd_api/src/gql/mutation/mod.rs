use async_graphql::{Context, Enum, Interface, Object, SimpleObject};

use crate::{
    gql::run_usecase,
    usecase::{SubscribeFeed, SubscribeFeedError, UnsubscribeFeed},
};

pub mod subscribe_feed;
pub mod unsubscribe_feed;

#[derive(Enum, PartialEq, Eq, Clone, Copy)]
pub enum ResponseCode {
    /// Operation success
    Ok,
    /// Principal does not have enough permissions
    Unauthorized,
    /// Given url is not valid feed url
    InvalidFeedUrl,
    /// Something went wrong
    InternalError,
}

#[derive(SimpleObject, Clone)]
pub struct ResponseStatus {
    code: ResponseCode,
}

impl ResponseStatus {
    fn ok() -> Self {
        ResponseStatus {
            code: ResponseCode::Ok,
        }
    }

    #[allow(unused)]
    fn unauthorized() -> Self {
        ResponseStatus {
            code: ResponseCode::Unauthorized,
        }
    }

    fn invalid_feed_url() -> Self {
        Self {
            code: ResponseCode::InvalidFeedUrl,
        }
    }

    fn internal() -> Self {
        Self {
            code: ResponseCode::InternalError,
        }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Interface)]
#[graphql(field(name = "status", method = "status", ty = "ResponseStatus"))]
enum MutationResponse {
    SubscribeFeed(subscribe_feed::SubscribeFeedSuccess),
    UnsubscribeFeed(unsubscribe_feed::UnsubscribeFeedSuccess),
}

#[derive(Interface)]
#[graphql(
    field(name = "status", ty = "ResponseStatus"),
    field(name = "message", ty = "String")
)]
enum ErrorResponse {
    SubscribeFeed(subscribe_feed::SubscribeFeedError),
    UnsubscribeFeed(unsubscribe_feed::UnsubscribeFeedError),
}

pub struct Mutation;

#[Object]
impl Mutation {
    /// Subscribe feed
    async fn subscribe_feed(
        &self,
        cx: &Context<'_>,
        input: subscribe_feed::SubscribeFeedInput,
    ) -> async_graphql::Result<subscribe_feed::SubscribeFeedResponse> {
        run_usecase!(SubscribeFeed, cx, input, |err: SubscribeFeedError| Ok(
            err.into()
        ))
    }

    /// Unsubscribe feed
    /// If given feed is not subscribed, this mutation will succeed
    async fn unsubscribe_feed(
        &self,
        cx: &Context<'_>,
        input: unsubscribe_feed::UnsubscribeFeedInput,
    ) -> async_graphql::Result<unsubscribe_feed::UnsubscribeFeedResponse> {
        run_usecase!(UnsubscribeFeed, cx, input, |err: anyhow::Error| Ok(
            err.into()
        ))
    }
}
