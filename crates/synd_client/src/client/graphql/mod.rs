use std::fmt::Debug;

use graphql_client::Response;
use serde::{Serialize, de::DeserializeOwned};
use synd_support::o11y::opentelemetry::extension::OpenTelemetrySpanExt as _;
use tracing::{Span, debug, warn};

use super::Client;
use crate::SyndApiError;

mod feed;
mod timeline;

const GRAPHQL_PATH: &str = "/graphql";

#[derive(Debug, Serialize)]
pub(super) struct GraphqlRequest<V> {
    query: &'static str,
    variables: V,
}

impl<V> GraphqlRequest<V> {
    pub(super) fn new(query: &'static str, variables: V) -> Self {
        Self { query, variables }
    }
}

pub(super) enum GraphqlOutcome<T> {
    Complete(T),
    Partial {
        data: T,
        errors: Vec<graphql_client::Error>,
    },
    Rejected {
        errors: Vec<graphql_client::Error>,
    },
    Empty,
}

/// GraphQL data accepted by an operation that permits partial results.
pub(super) enum AcceptedGraphqlData<T> {
    Complete(T),
    Partial {
        data: T,
        errors: Vec<graphql_client::Error>,
    },
}

impl<T> GraphqlOutcome<T> {
    pub(super) fn require_complete(self) -> Result<T, SyndApiError> {
        match self {
            Self::Complete(data) => Ok(data),
            Self::Partial { errors, .. } | Self::Rejected { errors } => {
                Err(SyndApiError::Graphql { errors })
            }
            Self::Empty => Err(SyndApiError::UnexpectedResponse {
                context: "response does not contain data and errors",
            }),
        }
    }

    pub(super) fn accept_partial(self) -> Result<AcceptedGraphqlData<T>, SyndApiError> {
        match self {
            Self::Complete(data) => Ok(AcceptedGraphqlData::Complete(data)),
            Self::Partial { data, errors } => Ok(AcceptedGraphqlData::Partial { data, errors }),
            Self::Rejected { errors } => Err(SyndApiError::Graphql { errors }),
            Self::Empty => Err(SyndApiError::UnexpectedResponse {
                context: "response does not contain data and errors",
            }),
        }
    }
}

impl<T> AcceptedGraphqlData<T> {
    pub(super) fn warn_partial_errors(&self) {
        if let Self::Partial { errors, .. } = self {
            warn!(?errors, "GraphQL query returned partial errors");
        }
    }

    pub(super) fn into_data(self) -> T {
        match self {
            Self::Complete(data) | Self::Partial { data, .. } => data,
        }
    }
}

impl<T> From<Response<T>> for GraphqlOutcome<T> {
    fn from(response: Response<T>) -> Self {
        let errors = response.errors.unwrap_or_default();
        match (response.data, errors.is_empty()) {
            (Some(data), true) => Self::Complete(data),
            (Some(data), false) => Self::Partial { data, errors },
            (None, false) => Self::Rejected { errors },
            (None, true) => Self::Empty,
        }
    }
}

impl Client {
    pub(super) async fn execute_graphql<Variables, ResponseData>(
        &self,
        request: &GraphqlRequest<Variables>,
    ) -> Result<GraphqlOutcome<ResponseData>, SyndApiError>
    where
        Variables: Serialize + Debug,
        ResponseData: DeserializeOwned + Debug,
    {
        let mut request = self
            .client
            .post(self.endpoint.join(GRAPHQL_PATH)?)
            .json(request)
            .build()
            .map_err(SyndApiError::BuildRequest)?;
        self.authentication
            .apply_authorization_header(request.headers_mut())?;

        synd_support::o11y::opentelemetry::http::inject_with_baggage(
            &Span::current().context(),
            request.headers_mut(),
            std::iter::once(synd_support::o11y::request_id_key_value()),
        );

        debug!(url = request.url().as_str(), "Send request");

        let response: Response<ResponseData> = self
            .client
            .execute(request)
            .await
            .map_err(SyndApiError::from_send_error)?
            .error_for_status()
            .map_err(SyndApiError::from_status_error)?
            .json()
            .await
            .map_err(SyndApiError::DecodeResponse)?;

        Ok(response.into())
    }
}
