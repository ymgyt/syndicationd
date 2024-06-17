use async_graphql::{InputObject, Object, Union};
use synd_feed::types::FeedUrl;

use crate::{gql::mutation::ResponseStatus, usecase};

#[derive(InputObject)]
pub struct UnsubscribeFeedInput {
    /// Feed url to unsubscribe
    pub url: FeedUrl,
}

impl From<UnsubscribeFeedInput> for usecase::UnsubscribeFeedInput {
    fn from(value: UnsubscribeFeedInput) -> Self {
        usecase::UnsubscribeFeedInput { url: value.url }
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Union)]
pub(crate) enum UnsubscribeFeedResponse {
    Success(UnsubscribeFeedSuccess),
    Error(UnsubscribeFeedError),
}

pub(crate) struct UnsubscribeFeedSuccess {
    pub status: ResponseStatus,
}

#[Object]
impl UnsubscribeFeedSuccess {
    pub async fn status(&self) -> ResponseStatus {
        self.status.clone()
    }
}

pub(crate) struct UnsubscribeFeedError {
    pub status: ResponseStatus,
    pub message: String,
}

#[Object]
impl UnsubscribeFeedError {
    pub async fn status(&self) -> ResponseStatus {
        self.status.clone()
    }

    /// Error message
    pub async fn message(&self) -> String {
        self.message.clone()
    }
}

impl From<anyhow::Error> for UnsubscribeFeedResponse {
    fn from(err: anyhow::Error) -> Self {
        UnsubscribeFeedResponse::Error(UnsubscribeFeedError {
            status: ResponseStatus::internal(),
            message: format!("{err}"),
        })
    }
}

impl From<usecase::Output<usecase::UnsubscribeFeedOutput>> for UnsubscribeFeedResponse {
    fn from(_output: usecase::Output<usecase::UnsubscribeFeedOutput>) -> Self {
        UnsubscribeFeedResponse::Success(UnsubscribeFeedSuccess {
            status: ResponseStatus::ok(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_error() {
        let r = UnsubscribeFeedResponse::from(anyhow::anyhow!("error"));
        assert!(
            matches!(r, UnsubscribeFeedResponse::Error(UnsubscribeFeedError { status, ..}) if status == ResponseStatus::internal())
        );
    }
}
