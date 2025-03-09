use std::{net::IpAddr, time::Duration};

use axum::{
    BoxError, Extension, Router,
    error_handling::HandleErrorLayer,
    http::{StatusCode, header::AUTHORIZATION},
    response::IntoResponse,
    routing::{get, post},
};
use tokio::net::TcpListener;
use tokio_metrics::TaskMonitor;
use tower::{ServiceBuilder, limit::ConcurrencyLimitLayer, timeout::TimeoutLayer};
use tower_http::{
    cors::CorsLayer, limit::RequestBodyLimitLayer, sensitive_headers::SetSensitiveHeadersLayer,
};
use tracing::info;

use crate::{
    config,
    dependency::Dependency,
    gql::{self, SyndSchema},
    serve::layer::{authenticate, request_metrics::RequestMetricsLayer, trace},
    shutdown::Shutdown,
};

pub mod auth;
mod probe;

pub mod layer;

pub struct BindOptions {
    pub port: u16,
    pub addr: IpAddr,
}

pub struct ServeOptions {
    pub timeout: Duration,
    pub body_limit_bytes: usize,
    pub concurrency_limit: usize,
}

#[derive(Clone)]
pub(crate) struct Context {
    pub gql_monitor: TaskMonitor,
    pub schema: SyndSchema,
}

/// Bind tcp listener and serve.
pub async fn listen_and_serve(
    dep: Dependency,
    bind: BindOptions,
    shutdown: Shutdown,
) -> anyhow::Result<()> {
    info!(addr = %bind.addr, port = bind.port, "Listening...");
    let listener = TcpListener::bind((bind.addr, bind.port)).await?;

    serve(listener, dep, shutdown).await
}

/// Start api server
pub async fn serve(
    listener: TcpListener,
    dep: Dependency,
    shutdown: Shutdown,
) -> anyhow::Result<()> {
    let Dependency {
        authenticator,
        runtime,
        tls_config,
        serve_options:
            ServeOptions {
                timeout: request_timeout,
                body_limit_bytes: request_body_limit_bytes,
                concurrency_limit,
            },
        monitors,
    } = dep;

    let cx = Context {
        gql_monitor: monitors.graphql_task_monitor(),
        schema: gql::schema_builder().data(runtime).finish(),
    };

    tokio::spawn(monitors.emit_metrics(
        config::metrics::MONITOR_INTERVAL,
        shutdown.cancellation_token(),
    ));

    let service = Router::new()
        .route("/graphql", post(gql::handler::graphql))
        .layer(Extension(cx))
        .layer(authenticate::AuthenticateLayer::new(authenticator))
        .route("/graphql", get(gql::handler::graphiql))
        .layer(
            ServiceBuilder::new()
                .layer(SetSensitiveHeadersLayer::new(std::iter::once(
                    AUTHORIZATION,
                )))
                .layer(trace::layer())
                .layer(HandleErrorLayer::new(handle_middleware_error))
                .layer(TimeoutLayer::new(request_timeout))
                .layer(ConcurrencyLimitLayer::new(concurrency_limit))
                .layer(RequestBodyLimitLayer::new(request_body_limit_bytes))
                .layer(CorsLayer::new()),
        )
        .route(config::serve::HEALTH_CHECK_PATH, get(probe::healthcheck))
        .layer(RequestMetricsLayer::new())
        .fallback(not_found);

    tracing::info!("Serving...");

    axum_server::from_tcp_rustls(listener.into_std()?, tls_config)
        .handle(shutdown.into_handle())
        .serve(service.into_make_service())
        .await?;

    tracing::info!("Shutdown complete");

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

async fn not_found() -> impl IntoResponse {
    StatusCode::NOT_FOUND
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn error_mapping() {
        assert_eq!(
            handle_middleware_error(Box::new(tower::timeout::error::Elapsed::new()))
                .await
                .0,
            StatusCode::REQUEST_TIMEOUT
        );
        assert_eq!(
            handle_middleware_error(Box::new(std::io::Error::from(
                std::io::ErrorKind::OutOfMemory
            )))
            .await
            .0,
            StatusCode::INTERNAL_SERVER_ERROR,
        );

        assert_eq!(
            not_found().await.into_response().status(),
            StatusCode::NOT_FOUND
        );
    }
}
