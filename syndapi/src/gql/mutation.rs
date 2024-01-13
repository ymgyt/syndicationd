use async_graphql::{Context, Enum, InputObject, Interface, Object, SimpleObject, Union};

use crate::{
    gql::object::FeedMeta,
    principal::Principal,
    usecase::{
        authorize::Authorizer,
        subscribe_feed::{SubscribeFeed, SubscribeFeedInput, SubscribeFeedOutput},
        Input, MakeUsecase, Output, Usecase,
    },
};

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
}

impl ResponseStatus {
    fn ok() -> Self {
        ResponseStatus {
            code: ResponseCode::Ok,
        }
    }

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

pub mod subscribe_feed {
    use crate::gql::object::FeedMeta;

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
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn subscribe_feed(
        &self,
        cx: &Context<'_>,
        input: subscribe_feed::SubscribeFeedInput,
    ) -> async_graphql::Result<subscribe_feed::SubscribeFeedResponse> {
        let input = SubscribeFeedInput { url: input.url };
        let (principal, usecase) = authorize!(
            cx,
            SubscribeFeed,
            &input,
            subscribe_feed::SubscribeFeedResponse
        );

        let Output {
            output: SubscribeFeedOutput { feed },
        } = usecase.usecase(Input { principal, input }).await?;

        Ok(subscribe_feed::SubscribeFeedResponse::Success(
            subscribe_feed::SubscribeFeedSuccess {
                status: ResponseStatus::ok(),
                feed: FeedMeta::from(feed),
            },
        ))
    }
}

// Extract usecase and exec authorization
macro_rules! authorize {
    ($cx:ident, $usecase:ty, $input:expr, $response:ty) => {{
        let uc = $cx.data_unchecked::<MakeUsecase>().make::<$usecase>();
        let principal = $cx.data_unchecked::<Principal>().clone();
        let authorizer = $cx.data_unchecked::<Authorizer>();

        match authorizer.authorize(principal, &uc, $input).await {
            Ok(authorized_principal) => (authorized_principal, uc),
            Err(_unauthorized) => return Ok(<$response>::from(ResponseStatus::unauthorized())),
        }
    }};
}

pub(super) use authorize;
