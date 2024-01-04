use async_graphql::{Context, Object};
use synd;

pub struct Subscription {}
pub struct Feed(synd::Feed);

#[Object]
impl Feed {
    async fn url(&self) -> String {
        self.0.url.clone()
    }
}

#[Object]
impl Subscription {
    async fn feeds(&self) -> Vec<Feed> {
        vec![
            Feed(synd::Feed::new("foo".into())),
            Feed(synd::Feed::new("bar".into())),
        ]
    }
}

pub struct Query;

#[Object]
impl Query {
    async fn subscription(&self, _ctx: &Context<'_>) -> Subscription {
        Subscription {}
    }
}
