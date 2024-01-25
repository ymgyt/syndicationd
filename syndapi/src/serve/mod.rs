use std::time::Duration;

use async_graphql::{extensions::Tracing, EmptySubscription, Schema};
use axum::{
    http::header::AUTHORIZATION,
    middleware,
    routing::{get, post},
    Extension, Router,
};
use tokio::net::TcpListener;
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, sensitive_headers::SetSensitiveHeadersLayer,
    timeout::TimeoutLayer,
};
use tracing::info;

use crate::{
    config,
    dependency::Dependency,
    gql::{self, Mutation, Query},
    serve::layer::trace,
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

    let schema = Schema::build(Query, Mutation, EmptySubscription)
        .data(runtime)
        .extension(Tracing)
        .finish();

    let service = Router::new()
        .route("/graphql", post(gql::handler::graphql))
        .layer(Extension(schema))
        .route_layer(middleware::from_fn_with_state(
            authenticator,
            auth::authenticate,
        ))
        .layer(
            // applied top to bottom
            tower::ServiceBuilder::new()
                .layer(SetSensitiveHeadersLayer::new(std::iter::once(
                    AUTHORIZATION,
                )))
                .layer(RequestBodyLimitLayer::new(2048))
                .layer(trace::layer())
                .layer(TimeoutLayer::new(Duration::from_secs(15)))
                .layer(CorsLayer::new()),
        )
        .route("/graphql", get(gql::handler::graphiql))
        .route("/healthcheck", get(probe::healthcheck));

    // TODO: graceful shutdown
    axum::serve(listener, service).await?;
    Ok(())
}
