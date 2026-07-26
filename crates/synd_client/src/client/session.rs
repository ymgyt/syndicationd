use std::fmt::Debug;

use reqwest::StatusCode;
use serde::{Serialize, de::DeserializeOwned};
use synd_protocol::session::{
    CloseSessionErrorResponse, CloseSessionRequest, CloseSessionResponse, OpenSessionErrorResponse,
    OpenSessionRequest, OpenSessionResponse, RenewSessionErrorResponse, RenewSessionRequest,
    RenewSessionResponse,
};

use super::Client;
use crate::SyndApiError;

enum SessionOutcome<T, E> {
    Accepted(T),
    Rejected(E),
}

trait SessionRequest: Serialize + Debug {
    type Response: DeserializeOwned;
    type Rejection: DeserializeOwned;

    const PATH: &'static str;
    const REJECTION_STATUS: StatusCode;
    const UNEXPECTED_RESPONSE_CONTEXT: &'static str;

    fn rejected(response: Self::Rejection) -> SyndApiError;

    async fn decode_response(
        response: reqwest::Response,
    ) -> Result<SessionOutcome<Self::Response, Self::Rejection>, SyndApiError> {
        let status = response.status();

        if status.is_success() {
            return response
                .json()
                .await
                .map(SessionOutcome::Accepted)
                .map_err(SyndApiError::DecodeResponse);
        }
        if status == Self::REJECTION_STATUS {
            return response
                .json()
                .await
                .map(SessionOutcome::Rejected)
                .map_err(SyndApiError::DecodeResponse);
        }

        response
            .error_for_status()
            .map_err(SyndApiError::from_status_error)?;
        Err(SyndApiError::UnexpectedResponse {
            context: Self::UNEXPECTED_RESPONSE_CONTEXT,
        })
    }
}

impl SessionRequest for OpenSessionRequest {
    type Response = OpenSessionResponse;
    type Rejection = OpenSessionErrorResponse;

    const PATH: &'static str = "/session/open";
    const REJECTION_STATUS: StatusCode = StatusCode::CONFLICT;
    const UNEXPECTED_RESPONSE_CONTEXT: &'static str = "session open";

    fn rejected(response: Self::Rejection) -> SyndApiError {
        SyndApiError::OpenSession(response)
    }
}

impl SessionRequest for RenewSessionRequest {
    type Response = RenewSessionResponse;
    type Rejection = RenewSessionErrorResponse;

    const PATH: &'static str = "/session/renew";
    const REJECTION_STATUS: StatusCode = StatusCode::NOT_FOUND;
    const UNEXPECTED_RESPONSE_CONTEXT: &'static str = "session renew";

    fn rejected(response: Self::Rejection) -> SyndApiError {
        SyndApiError::RenewSession(response)
    }
}

impl SessionRequest for CloseSessionRequest {
    type Response = CloseSessionResponse;
    type Rejection = CloseSessionErrorResponse;

    const PATH: &'static str = "/session/close";
    const REJECTION_STATUS: StatusCode = StatusCode::NOT_FOUND;
    const UNEXPECTED_RESPONSE_CONTEXT: &'static str = "session close";

    fn rejected(response: Self::Rejection) -> SyndApiError {
        SyndApiError::CloseSession(response)
    }
}

impl Client {
    pub async fn open_session(
        &self,
        request: OpenSessionRequest,
    ) -> Result<OpenSessionResponse, SyndApiError> {
        self.execute_session(request).await
    }

    pub async fn close_session(
        &self,
        request: CloseSessionRequest,
    ) -> Result<CloseSessionResponse, SyndApiError> {
        self.execute_session(request).await
    }

    pub async fn renew_session(
        &self,
        request: RenewSessionRequest,
    ) -> Result<RenewSessionResponse, SyndApiError> {
        self.execute_session(request).await
    }

    async fn execute_session<R>(&self, request: R) -> Result<R::Response, SyndApiError>
    where
        R: SessionRequest,
    {
        let response = self.send_session_request(&request).await?;
        let outcome = R::decode_response(response).await?;

        match outcome {
            SessionOutcome::Accepted(response) => Ok(response),
            SessionOutcome::Rejected(rejection) => Err(R::rejected(rejection)),
        }
    }

    async fn send_session_request<R>(&self, request: &R) -> Result<reqwest::Response, SyndApiError>
    where
        R: SessionRequest,
    {
        let mut request = self
            .client
            .post(self.endpoint.join(R::PATH)?)
            .json(request)
            .build()
            .map_err(SyndApiError::BuildRequest)?;
        self.authentication
            .apply_authorization_header(request.headers_mut())?;
        self.client
            .execute(request)
            .await
            .map_err(SyndApiError::from_send_error)
    }
}
