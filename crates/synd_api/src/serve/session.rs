use axum::{
    Extension, Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use synd_protocol::session::{CloseSessionRequest, OpenSessionRequest, RenewSessionRequest};

use crate::session::DaemonSessions;

pub(super) async fn open(
    Extension(sessions): Extension<DaemonSessions>,
    Json(request): Json<OpenSessionRequest>,
) -> Response {
    match sessions.open(&request) {
        Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
        Err(error) => (StatusCode::CONFLICT, Json(error)).into_response(),
    }
}

pub(super) async fn close(
    Extension(sessions): Extension<DaemonSessions>,
    Json(request): Json<CloseSessionRequest>,
) -> Response {
    match sessions.close(&request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::NOT_FOUND, Json(error)).into_response(),
    }
}

pub(super) async fn renew(
    Extension(sessions): Extension<DaemonSessions>,
    Json(request): Json<RenewSessionRequest>,
) -> Response {
    match sessions.renew(&request) {
        Ok(response) => Json(response).into_response(),
        Err(error) => (StatusCode::NOT_FOUND, Json(error)).into_response(),
    }
}
