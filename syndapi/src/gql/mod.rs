mod query;

use async_graphql::{EmptyMutation, EmptySubscription, Schema, SchemaBuilder};
pub use query::Query;

pub type SyndSchema = Schema<Query, EmptyMutation, EmptySubscription>;

pub fn schema() -> SchemaBuilder<Query, EmptyMutation, EmptySubscription> {
    Schema::build(Query, EmptyMutation, EmptySubscription)
}

pub mod handler {
    use async_graphql::http::GraphiQLSource;
    use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
    use axum::{response::IntoResponse, Extension};

    use crate::principal::Principal;

    use super::SyndSchema;

    pub async fn graphiql() -> impl IntoResponse {
        axum::response::Html(GraphiQLSource::build().endpoint("/gql").finish())
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
