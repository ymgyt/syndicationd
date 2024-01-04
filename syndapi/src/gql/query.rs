use async_graphql::{Context, Object};
use synd;
use tracing::info;

use crate::{persistence::Datastore, principal::Principal};

pub struct Subscription<'a> {
    user_id: &'a str,
}
pub struct Feed(synd::Feed);

#[Object]
impl Feed {
    async fn url(&self) -> String {
        self.0.url.clone()
    }
}

#[Object]
impl<'a> Subscription<'a> {
    async fn feeds(&self, cx: &Context<'_>) -> Vec<Feed> {
        let d = cx.data_unchecked::<Datastore>();
        d.fetch_subscription_feeds(self.user_id)
            .await
            .unwrap()
            .into_iter()
            .map(Feed)
            .collect()
    }
}

pub struct Query;

#[Object]
impl Query {
    async fn subscription<'cx>(&self, cx: &Context<'cx>) -> Subscription<'cx> {
        let Principal::User(user) = cx.data_unchecked::<Principal>();
        info!("Query subscription {user:?}");

        Subscription { user_id: user.id() }
    }
}
