use async_graphql::extensions::Tracing;
use axum::{middleware, routing::get, Extension, Router};
use tokio::net::TcpListener;
use tracing::info;

use crate::{gql, persistence::Datastore};

use self::auth::Authenticator;

mod auth;
mod probe;

/// Bind tcp listener and serve.
pub async fn listen_and_serve() -> anyhow::Result<()> {
    let addr = "0.0.0.0:5959";
    let listener = TcpListener::bind(addr).await?;

    info!(addr, "Listening...");

    serve(listener).await
}

/// Start api server
pub async fn serve(listener: TcpListener) -> anyhow::Result<()> {
    let datastore = Datastore::new()?;
    let schema = gql::schema().data(datastore).extension(Tracing).finish();

    let authenticator = Authenticator::new()?;

    let service = Router::new()
        .route(
            "/gql",
            get(gql::handler::graphiql).post(gql::handler::graphql),
        )
        .layer(Extension(schema))
        .route_layer(middleware::from_fn_with_state(
            authenticator,
            auth::authenticate,
        ))
        .route("/healthcheck", get(probe::healthcheck));

    // TODO: graceful shutdown
    axum::serve(listener, service).await?;
    Ok(())
}
