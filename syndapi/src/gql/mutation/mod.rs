use async_graphql::{Context, Enum, Interface, Object, SimpleObject};

use crate::{gql::run_usecase, usecase::SubscribeFeed};

pub mod subscribe_feed;

#[derive(Enum, PartialEq, Eq, Clone, Copy)]
pub enum ResponseCode {
    /// Operation success
    Ok,
    /// Principal does not have enough permissions
    Unauthorized,
    /// Something went wrong
    InternalError,
}

#[derive(SimpleObject, Clone)]
pub struct ResponseStatus {
    code: ResponseCode,
    // TODO: add message
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
}

#[derive(Interface)]
#[graphql(field(name = "status", method = "status", ty = "ResponseStatus"))]
enum MutationResponse {
    SubscribeFeed(subscribe_feed::SubscribeFeedSuccess),
}

#[derive(Interface)]
#[graphql(
    field(name = "status", ty = "ResponseStatus"),
    field(name = "message", ty = "String")
)]
enum ErrorResponse {
    SubscribeFeed(subscribe_feed::SubscribeFeedError),
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn subscribe_feed(
        &self,
        cx: &Context<'_>,
        input: subscribe_feed::SubscribeFeedInput,
    ) -> async_graphql::Result<subscribe_feed::SubscribeFeedResponse> {
        run_usecase!(SubscribeFeed, cx, input)
    }
}
