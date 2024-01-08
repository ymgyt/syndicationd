use async_graphql::{Context, InputObject, Object};

use crate::{persistence::Datastore, principal::Principal};

pub struct Mutation;

#[derive(InputObject)]
pub struct SubscribeFeedInput {
    url: String,
}

#[Object]
impl Mutation {
    async fn subscribe_feed(
        &self,
        cx: &Context<'_>,
        input: SubscribeFeedInput,
    ) -> async_graphql::Result<String> {
        let Principal::User(user) = cx.data_unchecked::<Principal>();

        let datastore = cx.data_unchecked::<Datastore>();
        datastore
            .add_feed_to_subscription(user.id(), input.url)
            .await?;
        Ok("OK".into())
    }
}
