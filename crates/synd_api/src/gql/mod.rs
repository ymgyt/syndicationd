mod query;
pub(crate) use query::Query;

mod mutation;
use async_graphql::{Schema, SchemaBuilder};
pub(crate) use mutation::Mutation;

mod subscription;
pub(crate) use subscription::RegistrySubscription;

use crate::{dependency::LiveFeedRegistry, principal::Principal};
use synd_registry::SubscriberId;

pub(crate) mod object;
pub(crate) mod scalar;

pub(crate) type SyndSchema = Schema<Query, Mutation, RegistrySubscription>;

pub(crate) mod handler {
    use async_graphql::{Data, http::GraphiQLSource};
    use async_graphql_axum::{GraphQLProtocol, GraphQLRequest, GraphQLResponse, GraphQLWebSocket};
    use axum::{Extension, extract::WebSocketUpgrade, response::IntoResponse};
    use synd_support::o11y::audit_span;
    use tokio_metrics::TaskMonitor;
    use tracing::Instrument;

    use crate::{principal::Principal, serve::Context};

    pub(crate) async fn graphiql() -> impl IntoResponse {
        axum::response::Html(
            GraphiQLSource::build()
                .endpoint("/graphql")
                .subscription_endpoint("/graphql/ws")
                .finish(),
        )
    }

    pub(crate) async fn graphql(
        Extension(Context {
            schema,
            gql_monitor,
        }): Extension<Context>,
        Extension(principal): Extension<Principal>,
        req: GraphQLRequest,
    ) -> GraphQLResponse {
        let req = req.into_inner().data(principal);
        TaskMonitor::instrument(&gql_monitor, schema.execute(req).instrument(audit_span!()))
            .await
            .into()
    }

    pub(crate) async fn graphql_ws(
        Extension(Context { schema, .. }): Extension<Context>,
        Extension(principal): Extension<Principal>,
        protocol: GraphQLProtocol,
        ws: WebSocketUpgrade,
    ) -> impl IntoResponse {
        let mut data = Data::default();
        data.insert(principal);

        ws.protocols(async_graphql::http::ALL_WEBSOCKET_PROTOCOLS)
            .on_upgrade(move |stream| {
                GraphQLWebSocket::new(stream, schema, protocol)
                    .with_data(data)
                    .serve()
                    .instrument(audit_span!())
            })
    }
}

#[must_use]
pub(crate) fn schema_builder() -> SchemaBuilder<Query, Mutation, RegistrySubscription> {
    let schema = Schema::build(Query, Mutation, RegistrySubscription);

    if cfg!(not(feature = "introspection")) {
        schema
            .disable_introspection()
            .limit_depth(10)
            .limit_complexity(80)
    } else {
        schema.limit_depth(20).limit_complexity(300)
    }
}

pub(crate) fn principal(cx: &async_graphql::Context<'_>) -> Principal {
    cx.data_unchecked::<Principal>().clone()
}

pub(crate) fn registry<'a>(cx: &'a async_graphql::Context<'_>) -> &'a LiveFeedRegistry {
    cx.data_unchecked::<LiveFeedRegistry>()
}

pub(crate) fn subscriber_id(cx: &async_graphql::Context<'_>) -> SubscriberId {
    let principal = principal(cx);
    SubscriberId::new(principal.principal_id())
}
