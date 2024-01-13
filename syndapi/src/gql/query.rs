use std::sync::Arc;

use async_graphql::{
    connection::{Connection, Edge},
    Context, Object, Result,
};

use crate::{gql::object::FeedMeta, persistence::Datastore, principal::Principal};

pub struct Subscription<'a> {
    user_id: &'a str,
}

pub struct Resolver {
    pub datastore: Arc<dyn Datastore>,
}

#[Object]
impl<'a> Subscription<'a> {
    async fn feeds(&self, cx: &Context<'_>) -> Result<Connection<usize, FeedMeta>> {
        let r = cx.data_unchecked::<Resolver>();
        let mut connection = Connection::new(false, false);
        connection.edges.extend(
            r.datastore
                .fetch_subscription_feeds(self.user_id)
                .await
                .unwrap()
                .into_iter()
                .map(FeedMeta::from)
                .enumerate()
                .map(|(idx, feed)| Edge::new(idx, feed)),
        );
        Ok(connection)
    }
}

pub struct Query;

#[Object]
impl Query {
    async fn subscription<'cx>(&self, cx: &Context<'cx>) -> Subscription<'cx> {
        let Principal::User(user) = cx.data_unchecked::<Principal>();

        Subscription { user_id: user.id() }
    }
}
