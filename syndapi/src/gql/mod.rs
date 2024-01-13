mod query;
pub use query::{Query, Resolver};

mod mutation;
use async_graphql::{EmptySubscription, Schema};
pub use mutation::Mutation;

pub mod object;

pub type SyndSchema = Schema<Query, Mutation, EmptySubscription>;

pub mod handler {
    use async_graphql::http::GraphiQLSource;
    use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
    use axum::{response::IntoResponse, Extension};

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
        schema.execute(req).await.into()
    }
}
