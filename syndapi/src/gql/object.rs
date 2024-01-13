use async_graphql::Object;

pub struct FeedMeta(synd::types::FeedMeta);

#[Object]
impl FeedMeta {
    async fn url(&self) -> &str {
        self.0.url.as_str()
    }

    async fn title(&self) -> &str {
        self.0.title.as_str()
    }
}

impl From<synd::types::FeedMeta> for FeedMeta {
    fn from(feed: synd::types::FeedMeta) -> Self {
        Self(feed)
    }
}
