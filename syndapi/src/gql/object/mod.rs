use async_graphql::{
    connection::{Connection, Edge},
    Enum, Object, SimpleObject, ID,
};
use feed_rs::model as feedrs;
use synd::types;

use crate::gql::scalar;

use self::id::FeedIdV1;

pub mod id;

#[derive(SimpleObject)]
pub struct Link {
    pub href: String,
    pub rel: Option<String>,
    pub media_type: Option<String>,
    pub href_lang: Option<String>,
    pub title: Option<String>,
}

impl From<feedrs::Link> for Link {
    fn from(value: feedrs::Link) -> Self {
        Self {
            href: value.href,
            rel: value.rel,
            media_type: value.media_type,
            href_lang: value.href_lang,
            title: value.title,
        }
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
    /// Feed Id
    async fn id(&self) -> ID {
        FeedIdV1::new(self.0.url()).into()
    }

    /// Undering feed specification
    async fn r#type(&self) -> FeedType {
        self.0.r#type().into()
    }

    /// Feed title
    async fn title(&self) -> Option<&str> {
        self.0.title()
    }

    /// Feed URL
    async fn url(&self) -> &str {
        self.0.url()
    }

    /// The time at which the feed was last modified
    async fn updated(&self) -> Option<scalar::Rfc3339Time> {
        self.0.updated().map(Into::into)
    }

    /// Feed authors
    async fn authors(&self) -> Connection<usize, String> {
        let mut c = Connection::new(false, false);
        c.edges.extend(
            self.0
                .authors()
                .enumerate()
                .map(|(idx, author)| Edge::new(idx, author.to_owned())),
        );

        c
    }

    /// Description of feed
    async fn description(&self) -> Option<&str> {
        self.0.description()
    }

    async fn links(&self) -> Connection<usize, Link> {
        let mut c = Connection::new(false, false);
        c.edges.extend(
            self.0
                .links()
                .map(|link| Link::from(link.clone()))
                .enumerate()
                .map(|(idx, link)| Edge::new(idx, link)),
        );

        c
    }

    // TODO: entries
    // TODO: category
}

impl From<types::Feed> for Feed {
    fn from(value: types::Feed) -> Self {
        Self(value)
    }
}
