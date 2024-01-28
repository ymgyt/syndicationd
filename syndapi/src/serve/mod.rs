use std::time::Duration;

use axum::{
    error_handling::HandleErrorLayer,
    http::{header::AUTHORIZATION, StatusCode},
    routing::{get, post},
    BoxError, Extension, Router,
};
use tokio::net::TcpListener;
use tower::{limit::ConcurrencyLimitLayer, timeout::TimeoutLayer, ServiceBuilder};
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, sensitive_headers::SetSensitiveHeadersLayer,
};
use tracing::info;

use crate::{
    config,
    dependency::Dependency,
    gql,
    serve::layer::{authenticate, trace},
};

pub mod auth;
mod probe;

pub mod layer;

/// Bind tcp listener and serve.
pub async fn listen_and_serve(dep: Dependency) -> anyhow::Result<()> {
    // should 127.0.0.1?
    let addr = ("0.0.0.0", config::PORT);
    let listener = TcpListener::bind(addr).await?;

    info!(ip = addr.0, port = addr.1, "Listening...");

    serve(listener, dep).await
}

/// Start api server
pub async fn serve(listener: TcpListener, dep: Dependency) -> anyhow::Result<()> {
    let Dependency {
        authenticator,
        runtime,
    } = dep;

    let schema = gql::schema_builder().data(runtime).finish();

    let service = Router::new()
        .route("/graphql", post(gql::handler::graphql))
        .layer(Extension(schema))
        .layer(authenticate::AuthenticateLayer::new(authenticator))
        .route("/graphql", get(gql::handler::graphiql))
        .layer(
            ServiceBuilder::new()
                .layer(SetSensitiveHeadersLayer::new(std::iter::once(
                    AUTHORIZATION,
                )))
                .layer(trace::layer())
                .layer(HandleErrorLayer::new(handle_middleware_error))
                .layer(TimeoutLayer::new(Duration::from_secs(20)))
                .layer(ConcurrencyLimitLayer::new(100))
                .layer(RequestBodyLimitLayer::new(2048))
                .layer(CorsLayer::new()),
        )
        .route("/healthcheck", get(probe::healthcheck));

    // TODO: graceful shutdown
    axum::serve(listener, service).await?;
    Ok(())
}

async fn handle_middleware_error(err: BoxError) -> (StatusCode, String) {
    if err.is::<tower::timeout::error::Elapsed>() {
        (
            StatusCode::REQUEST_TIMEOUT,
            "Request took too long".to_string(),
        )
    } else {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Unhandled internal error: {err}"),
        )
    }
}
