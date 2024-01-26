use std::{borrow::Cow, sync::Arc};

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

pub struct Entry(types::Entry);

#[Object]
impl Entry {
    /// Entry title
    async fn title(&self) -> Option<&str> {
        self.0.title()
    }

    /// The time at which the entry published
    async fn published(&self) -> Option<scalar::Rfc3339Time> {
        self.0.published().map(Into::into)
    }

    /// Entry summary
    async fn summary(&self) -> Option<&str> {
        self.0.summary()
    }
}

impl From<types::Entry> for Entry {
    fn from(value: types::Entry) -> Self {
        Self(value)
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

pub struct Feed(Arc<types::Feed>);

#[Object]
impl Feed {
    /// Feed Id
    async fn id(&self) -> ID {
        FeedIdV1::new(self.0.meta().url()).into()
    }

    /// Undering feed specification
    async fn r#type(&self) -> FeedType {
        self.0.meta().r#type().into()
    }

    /// Feed title
    async fn title(&self) -> Option<&str> {
        self.0.meta().title()
    }

    /// Feed URL
    async fn url(&self) -> &str {
        self.0.meta().url()
    }

    /// The time at which the feed was last modified
    async fn updated(&self) -> Option<scalar::Rfc3339Time> {
        self.0.meta().updated().map(Into::into)
    }

    /// Feed entries
    async fn entries(
        &self,
        #[graphql(default = 5)] first: Option<i32>,
    ) -> Connection<usize, Entry> {
        let first = first.unwrap_or(5).max(0) as usize;
        let entries = self
            .0
            .entries()
            .map(|entry| Entry(entry.clone()))
            .take(first)
            .collect::<Vec<_>>();

        let mut c = Connection::new(false, entries.len() > first);
        c.edges.extend(
            entries
                .into_iter()
                .enumerate()
                .map(|(idx, entry)| Edge::new(idx, entry)),
        );

        c
    }

    /// Feed authors
    async fn authors(&self) -> Connection<usize, String> {
        let mut c = Connection::new(false, false);
        c.edges.extend(
            self.0
                .meta()
                .authors()
                .enumerate()
                .map(|(idx, author)| Edge::new(idx, author.to_owned())),
        );

        c
    }

    /// Description of feed
    async fn description(&self) -> Option<&str> {
        self.0.meta().description()
    }

    async fn links(&self) -> Connection<usize, Link> {
        let mut c = Connection::new(false, false);
        c.edges.extend(
            self.0
                .meta()
                .links()
                .map(|link| Link::from(link.clone()))
                .enumerate()
                .map(|(idx, link)| Edge::new(idx, link)),
        );

        c
    }

    async fn website_url(&self) -> Option<&str> {
        self.0.meta().website_url()
    }
}

impl From<Arc<types::Feed>> for Feed {
    fn from(value: Arc<types::Feed>) -> Self {
        Self(value)
    }
}

pub(super) struct FeedMeta<'a>(Cow<'a, types::FeedMeta>);

#[Object]
impl<'a> FeedMeta<'a> {
    async fn title(&self) -> Option<&str> {
        self.0.title()
    }
}

impl From<types::FeedMeta> for FeedMeta<'static> {
    fn from(value: types::FeedMeta) -> Self {
        Self(Cow::Owned(value))
    }
}

/// Entry with feed metadata
pub(super) struct FeedEntry<'a> {
    pub feed: FeedMeta<'a>,
    pub entry: Entry,
}

#[Object]
impl<'a> FeedEntry<'a> {
    async fn feed(&self) -> &FeedMeta<'a> {
        &self.feed
    }

    async fn entry(&self) -> &Entry {
        &self.entry
    }
}
