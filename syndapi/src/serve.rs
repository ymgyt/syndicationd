use async_graphql::extensions::Tracing;
use axum::{middleware, routing::get, Extension, Router};
use tokio::net::TcpListener;
use tracing::info;

use crate::gql;

mod gql_handler {
    use async_graphql::http::GraphiQLSource;
    use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
    use axum::{response::IntoResponse, Extension};

    use crate::gql::SyndSchema;
    pub async fn graphiql() -> impl IntoResponse {
        axum::response::Html(GraphiQLSource::build().endpoint("/gql").finish())
    }

    pub async fn graphql(schema: Extension<SyndSchema>, req: GraphQLRequest) -> GraphQLResponse {
        schema.execute(req.into_inner()).await.into()
    }
}

mod auth {
    use std::convert::Infallible;

    use axum::{
        extract::Request,
        http::{self, StatusCode},
        middleware::Next,
        response::Response,
    };
    use tracing::warn;

    use crate::principal::Principal;

    /// Check authorization header and inject Authentication
    pub async fn authenticate(mut req: Request, next: Next) -> Result<Response, StatusCode> {
        let header = req
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|header| header.to_str().ok());

        let Some(token) = header else {
            return Err(StatusCode::UNAUTHORIZED);
        };
        let principal = match validate(token) {
            Ok(principal) => principal,
            Err(err) => {
                warn!("Invalid token {err}");
                return Err(StatusCode::UNAUTHORIZED);
            }
        };

        req.extensions_mut().insert(principal);
        Ok(next.run(req).await)
    }

    fn validate(token: &str) -> Result<Principal, Infallible> {
        Ok(Principal::User {
            name: token.to_owned(),
        })
    }
}

mod probe {
    pub async fn healthcheck() -> &'static str {
        "OK"
    }
}

/// Bind tcp listener and serve.
pub async fn listen_and_serve() -> anyhow::Result<()> {
    let addr = "0.0.0.0:5959";
    let listener = TcpListener::bind(addr).await?;

    info!(addr, "Listening...");

    serve(listener).await
}

/// Start api server
pub async fn serve(listener: TcpListener) -> anyhow::Result<()> {
    let schema = gql::schema().extension(Tracing).finish();

    let service = Router::new()
        .route(
            "/gql",
            get(gql_handler::graphiql).post(gql_handler::graphql),
        )
        .layer(Extension(schema))
        .route_layer(middleware::from_fn(auth::authenticate))
        .route("/healthcheck", get(probe::healthcheck));

    // TODO: graceful shutdown
    axum::serve(listener, service).await?;
    Ok(())
}
