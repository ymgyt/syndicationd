mod query;
pub use query::Query;

mod mutation;
use async_graphql::{extensions::Tracing, EmptySubscription, Schema, SchemaBuilder};
pub use mutation::Mutation;

use crate::{principal::Principal, usecase};

pub mod object;
pub mod scalar;

pub type SyndSchema = Schema<Query, Mutation, EmptySubscription>;

pub mod handler {
    use async_graphql::http::GraphiQLSource;
    use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
    use axum::{response::IntoResponse, Extension};
    use synd_o11y::audit_span;
    use tracing::Instrument;

    use crate::principal::Principal;

    use super::SyndSchema;

    pub async fn graphiql() -> impl IntoResponse {
        axum::response::Html(GraphiQLSource::build().endpoint("/graphql").finish())
    }

    pub async fn graphql(
        Extension(schema): Extension<SyndSchema>,
        Extension(principal): Extension<Principal>,
        req: GraphQLRequest,
    ) -> GraphQLResponse {
        // Inject authentication
        let req = req.into_inner().data(principal);
        schema.execute(req).instrument(audit_span!()).await.into()
    }
}

pub fn schema_builder() -> SchemaBuilder<Query, Mutation, EmptySubscription> {
    #[cfg(feature = "introspection")]
    let schema = Schema::build(Query, Mutation, EmptySubscription);
    #[cfg(not(feature = "introspection"))]
    let schema = Schema::build(Query, Mutation, EmptySubscription).disable_introspection();

    schema.extension(Tracing)
}

impl<'a> usecase::Context for &async_graphql::Context<'a> {
    fn principal(&self) -> Principal {
        self.data_unchecked::<Principal>().clone()
    }
}

impl<E> async_graphql::ErrorExtensions for usecase::Error<E>
where
    E: std::fmt::Display + Send + Sync + 'static,
{
    fn extend(&self) -> async_graphql::Error {
        async_graphql::Error::new(format!("{self}")).extend_with(|_, ext| match self {
            usecase::Error::Usecase(_) => ext.set("code", "TODO"),
            usecase::Error::Unauthorized(_) => ext.set("code", "UNAUTHORIZED"),
            usecase::Error::Datastore(_) => ext.set("code", "INTERNAL"),
        })
    }
}

macro_rules! run_usecase {
    ($usecase:ty, $cx:expr, $input:expr) => {{
        let runtime = $cx.data_unchecked::<crate::usecase::Runtime>();

        runtime
            .run::<$usecase, _, _>($cx, $input)
            .await
            .map_err(|err| async_graphql::ErrorExtensions::extend(&err))
            .map(Into::into)
    }};
}

pub(super) use run_usecase;
