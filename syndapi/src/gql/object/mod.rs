use async_graphql::{Enum, Object, ID};
use synd::types;

use self::id::FeedIdV1;

pub struct FeedMeta(types::FeedMeta);

pub mod id;

#[Object]
impl FeedMeta {
    async fn url(&self) -> &str {
        self.0.url.as_str()
    }

    async fn title(&self) -> &str {
        self.0.title.as_str()
    }
}

impl From<types::FeedMeta> for FeedMeta {
    fn from(feed: synd::types::FeedMeta) -> Self {
        Self(feed)
    }
}

#[derive(Enum, Clone, Copy, PartialEq, Eq)]
#[graphql(remote = "synd::types::FeedType")]
pub enum FeedType {
    Atom,
    RSS1,
    RSS2,
    RSS0,
    JSON,
}

pub struct Feed(types::Feed);

#[Object]
impl Feed {
    async fn id(&self) -> ID {
        FeedIdV1::new(self.0.url()).into()
    }

    async fn r#type(&self) -> FeedType {
        self.0.r#type().into()
    }
}

impl From<types::Feed> for Feed {
    fn from(value: types::Feed) -> Self {
        Self(value)
    }
}
