use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use synd_o11y::health_check::Health;

use crate::config;

pub async fn healthcheck() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, Health::CONTENT_TYPE)],
        Json(
            Health::pass()
                .with_version(config::app::VERSION)
                .with_description("health of synd-api"),
        ),
    )
}
