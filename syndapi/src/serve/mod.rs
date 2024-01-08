use async_graphql::extensions::Tracing;
use axum::{
    middleware,
    routing::{get, post},
    Extension, Router,
};
use tokio::net::TcpListener;
use tracing::info;

use crate::{gql, persistence::Datastore};

use self::auth::Authenticator;

pub mod auth;
mod probe;

pub struct Dependency {
    pub datastore: Datastore,
    pub authenticator: Authenticator,
}

/// Bind tcp listener and serve.
pub async fn listen_and_serve(dep: Dependency) -> anyhow::Result<()> {
    let addr = "0.0.0.0:5959";
    let listener = TcpListener::bind(addr).await?;

    info!(addr, "Listening...");

    serve(listener, dep).await
}

/// Start api server
pub async fn serve(listener: TcpListener, dep: Dependency) -> anyhow::Result<()> {
    let Dependency {
        datastore,
        authenticator,
    } = dep;

    let schema = gql::schema().data(datastore).extension(Tracing).finish();

    let service = Router::new()
        .route("/gql", post(gql::handler::graphql))
        .layer(Extension(schema))
        .route_layer(middleware::from_fn_with_state(
            authenticator,
            auth::authenticate,
        ))
        .route("/gql", get(gql::handler::graphiql))
        .route("/healthcheck", get(probe::healthcheck));

    // TODO: graceful shutdown
    axum::serve(listener, service).await?;
    Ok(())
}
