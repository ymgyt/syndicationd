use async_graphql::{Context, Enum, InputObject, Interface, Object, SimpleObject, Union};

use crate::{persistence::Datastore, principal::Principal};

#[derive(Enum, PartialEq, Eq, Clone, Copy)]
pub enum ResponseCode {
    /// Operation success
    Ok,
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

pub mod subscribe_feed {
    use super::*;

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
        pub url: String,
    }

    #[Object]
    impl SubscribeFeedSuccess {
        pub async fn status(&self) -> ResponseStatus {
            self.status.clone()
        }

        pub async fn url(&self) -> String {
            self.url.clone()
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
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn subscribe_feed(
        &self,
        cx: &Context<'_>,
        input: subscribe_feed::SubscribeFeedInput,
    ) -> async_graphql::Result<subscribe_feed::SubscribeFeedResponse> {
        let Principal::User(user) = cx.data_unchecked::<Principal>();

        let datastore = cx.data_unchecked::<Datastore>();
        datastore
            .add_feed_to_subscription(user.id(), input.url.clone())
            .await?;

        Ok(subscribe_feed::SubscribeFeedResponse::Success(
            subscribe_feed::SubscribeFeedSuccess {
                status: ResponseStatus::ok(),
                url: input.url,
            },
        ))
    }
}
