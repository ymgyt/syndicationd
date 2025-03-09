use std::{borrow::Cow, sync::Arc};

use async_graphql::{
    ID, Object, SimpleObject,
    connection::{Connection, ConnectionNameType, Edge, EdgeNameType, EmptyFields},
};
use feed_rs::model as feedrs;
use synd_feed::types::{self, Annotated, Category, FeedType, FeedUrl, Requirement};

use crate::gql::scalar;

use self::id::FeedIdV1;

pub mod id;

#[derive(SimpleObject)]
pub(crate) struct Link {
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

pub(crate) struct Entry<'a> {
    meta: Cow<'a, Annotated<types::FeedMeta>>,
    entry: types::Entry,
}

#[Object]
impl Entry<'_> {
    /// Feed of this entry
    async fn feed(&self) -> FeedMeta {
        self.meta.clone().into()
    }
    /// Entry title
    async fn title(&self) -> Option<&str> {
        self.entry.title()
    }

    /// Time at which the entry was last modified
    async fn updated(&self) -> Option<scalar::Rfc3339Time> {
        self.entry.updated().map(Into::into)
    }

    /// The time at which the entry published
    async fn published(&self) -> Option<scalar::Rfc3339Time> {
        self.entry.published().map(Into::into)
    }

    /// Entry summary. If there is no summary of the entry, return the content(is this bad api?)
    async fn summary(&self) -> Option<&str> {
        self.entry.summary().or(self.entry.content())
    }

    /// Link to websiteurl at which this entry is published
    async fn website_url(&self) -> Option<&str> {
        self.entry.website_url(self.meta.feed.r#type())
    }
}

impl<'a> Entry<'a> {
    pub fn new(meta: Cow<'a, Annotated<types::FeedMeta>>, entry: types::Entry) -> Self {
        Self { meta, entry }
    }
}

pub struct Feed(Annotated<Arc<types::Feed>>);

#[Object]
impl Feed {
    /// Feed Id
    async fn id(&self) -> ID {
        FeedIdV1::new(self.0.feed.meta().url()).into()
    }

    /// Undering feed specification
    async fn r#type(&self) -> FeedType {
        self.0.feed.meta().r#type()
    }

    /// Feed title
    async fn title(&self) -> Option<&str> {
        self.0.feed.meta().title()
    }

    /// Feed URL
    async fn url(&self) -> &FeedUrl {
        self.0.feed.meta().url()
    }

    /// The time at which the feed was last modified
    async fn updated(&self) -> Option<scalar::Rfc3339Time> {
        self.0.feed.meta().updated().map(Into::into)
    }

    /// Feed entries
    #[allow(clippy::cast_sign_loss)]
    async fn entries(
        &self,
        #[graphql(default = 5)] first: Option<i32>,
    ) -> Connection<
        usize,
        Entry,
        EmptyFields,
        EmptyFields,
        FeedEntryConnectionName,
        FeedEntryEdgeName,
    > {
        #[allow(clippy::cast_sign_loss)]
        let first = first.unwrap_or(5).max(0) as usize;
        let meta = self.0.project(|feed| feed.meta().clone());
        let entries = self
            .0
            .feed
            .entries()
            .map(move |entry| Entry::new(Cow::Owned(meta.clone()), entry.clone()))
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
                .feed
                .meta()
                .authors()
                .enumerate()
                .map(|(idx, author)| Edge::new(idx, author.to_owned())),
        );

        c
    }

    /// Description of feed
    async fn description(&self) -> Option<&str> {
        self.0.feed.meta().description()
    }

    async fn links(&self) -> Connection<usize, Link> {
        let mut c = Connection::new(false, false);
        c.edges.extend(
            self.0
                .feed
                .meta()
                .links()
                .map(|link| Link::from(link.clone()))
                .enumerate()
                .map(|(idx, link)| Edge::new(idx, link)),
        );

        c
    }

    async fn website_url(&self) -> Option<&str> {
        self.0.feed.meta().website_url()
    }

    async fn generator(&self) -> Option<&str> {
        self.0.feed.meta().generator()
    }

    /// Requirement level for feed
    async fn requirement(&self) -> Option<Requirement> {
        self.0.requirement
    }

    /// Feed category
    async fn category(&self) -> Option<&Category<'static>> {
        self.0.category.as_ref()
    }
}

pub struct FeedEntryConnectionName;

impl ConnectionNameType for FeedEntryConnectionName {
    fn type_name<T: async_graphql::OutputType>() -> String {
        "FeedEntryConnection".into()
    }
}

pub struct FeedEntryEdgeName;

impl EdgeNameType for FeedEntryEdgeName {
    fn type_name<T: async_graphql::OutputType>() -> String {
        "FeedEntryEdge".into()
    }
}

impl From<Annotated<Arc<types::Feed>>> for Feed {
    fn from(value: Annotated<Arc<types::Feed>>) -> Self {
        Self(value)
    }
}

pub(super) struct FeedMeta<'a>(Cow<'a, Annotated<types::FeedMeta>>);

#[Object]
impl FeedMeta<'_> {
    /// Title of the feed
    async fn title(&self) -> Option<&str> {
        self.0.feed.title()
    }

    /// Url of the feed
    async fn url(&self) -> &FeedUrl {
        self.0.feed.url()
    }

    /// Requirement Level for the feed
    async fn requirement(&self) -> Option<Requirement> {
        self.0.requirement
    }

    /// Category of the feed
    async fn category(&self) -> Option<&Category<'static>> {
        self.0.category.as_ref()
    }
}

impl<'a> From<Cow<'a, Annotated<types::FeedMeta>>> for FeedMeta<'a> {
    fn from(value: Cow<'a, Annotated<types::FeedMeta>>) -> Self {
        Self(value)
    }
}
