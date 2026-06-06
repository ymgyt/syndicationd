use axum::{
    Extension, Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use synd_protocol::session::{
    CloseSessionErrorResponse, CloseSessionRequest, CloseSessionResponse, OpenSessionErrorResponse,
    OpenSessionRequest, OpenSessionResponse, RenewSessionErrorResponse, RenewSessionRequest,
    RenewSessionResponse,
};

use crate::session::DaemonSessions;

pub(super) async fn open(
    Extension(sessions): Extension<DaemonSessions>,
    Json(request): Json<OpenSessionRequest>,
) -> Response {
    OpenSessionRouteResponse::from(sessions.open(&request)).into_response()
}

pub(super) async fn close(
    Extension(sessions): Extension<DaemonSessions>,
    Json(request): Json<CloseSessionRequest>,
) -> Response {
    CloseSessionRouteResponse::from(sessions.close(&request)).into_response()
}

pub(super) async fn renew(
    Extension(sessions): Extension<DaemonSessions>,
    Json(request): Json<RenewSessionRequest>,
) -> Response {
    RenewSessionRouteResponse::from(sessions.renew(&request)).into_response()
}

/// HTTP response selected for a daemon session open result.
#[derive(Debug)]
enum OpenSessionRouteResponse {
    Accepted(OpenSessionResponse),
    Rejected(OpenSessionErrorResponse),
}

impl From<Result<OpenSessionResponse, OpenSessionErrorResponse>> for OpenSessionRouteResponse {
    fn from(result: Result<OpenSessionResponse, OpenSessionErrorResponse>) -> Self {
        match result {
            Ok(response) => Self::Accepted(response),
            Err(error) => Self::Rejected(error),
        }
    }
}

impl IntoResponse for OpenSessionRouteResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Accepted(response) => (StatusCode::CREATED, Json(response)).into_response(),
            Self::Rejected(error) => (StatusCode::CONFLICT, Json(error)).into_response(),
        }
    }
}

/// HTTP response selected for a daemon session renew result.
#[derive(Debug)]
enum RenewSessionRouteResponse {
    Accepted(RenewSessionResponse),
    Rejected(RenewSessionErrorResponse),
}

impl From<Result<RenewSessionResponse, RenewSessionErrorResponse>> for RenewSessionRouteResponse {
    fn from(result: Result<RenewSessionResponse, RenewSessionErrorResponse>) -> Self {
        match result {
            Ok(response) => Self::Accepted(response),
            Err(error) => Self::Rejected(error),
        }
    }
}

impl IntoResponse for RenewSessionRouteResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Accepted(response) => (StatusCode::OK, Json(response)).into_response(),
            Self::Rejected(error) => (StatusCode::NOT_FOUND, Json(error)).into_response(),
        }
    }
}

/// HTTP response selected for a daemon session close result.
#[derive(Debug)]
enum CloseSessionRouteResponse {
    Accepted(CloseSessionResponse),
    Rejected(CloseSessionErrorResponse),
}

impl From<Result<CloseSessionResponse, CloseSessionErrorResponse>> for CloseSessionRouteResponse {
    fn from(result: Result<CloseSessionResponse, CloseSessionErrorResponse>) -> Self {
        match result {
            Ok(response) => Self::Accepted(response),
            Err(error) => Self::Rejected(error),
        }
    }
}

impl IntoResponse for CloseSessionRouteResponse {
    fn into_response(self) -> Response {
        match self {
            Self::Accepted(response) => (StatusCode::OK, Json(response)).into_response(),
            Self::Rejected(error) => (StatusCode::NOT_FOUND, Json(error)).into_response(),
        }
    }
}
