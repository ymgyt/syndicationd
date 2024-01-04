mod query;

use async_graphql::{EmptyMutation, EmptySubscription, Schema, SchemaBuilder};
pub use query::Query;

pub type SyndSchema = Schema<Query, EmptyMutation, EmptySubscription>;

pub fn schema() -> SchemaBuilder<Query, EmptyMutation, EmptySubscription> {
    Schema::build(Query, EmptyMutation, EmptySubscription)
}
