use std::{borrow::Cow, sync::Arc};

use async_graphql::{
    ID, Object, SimpleObject,
    connection::{Connection, ConnectionNameType, Edge, EdgeNameType, EmptyFields},
};
use synd_feed::{
    entry::{Content, Entry as SyndFeedEntry, EntryId},
    types::{self, Annotated, Category, FeedType, FeedUrl, Requirement},
};
use synd_registry::{entry::EntryAttrs as RegistryEntryAttrs, query::TimelineEntry};

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

impl From<&types::Link> for Link {
    fn from(value: &types::Link) -> Self {
        Self {
            href: value.href().to_owned(),
            rel: value.rel().map(ToOwned::to_owned),
            media_type: value.media_type().map(ToOwned::to_owned),
            href_lang: value.href_lang().map(ToOwned::to_owned),
            title: value.title().map(ToOwned::to_owned),
        }
    }
}

pub(crate) struct Entry {
    id: EntryId,
    meta: Annotated<types::FeedMeta>,
    body: EntryBody,
}

pub(crate) enum EntryBody {
    Feed(Box<SyndFeedEntry>),
    Timeline(RegistryEntryAttrs),
}

#[Object]
impl Entry {
    /// Entry id
    async fn id(&self) -> ID {
        self.id.as_str().into()
    }

    /// Feed of this entry
    async fn feed(&self) -> FeedMeta<'_> {
        Cow::Borrowed(&self.meta).into()
    }
    /// Entry title
    async fn title(&self) -> Option<&str> {
        match &self.body {
            EntryBody::Feed(entry) => entry.title().map(types::Text::content),
            EntryBody::Timeline(attrs) => attrs.title.as_deref(),
        }
    }

    /// Time at which the entry was last modified
    async fn updated(&self) -> Option<scalar::Rfc3339Time> {
        match &self.body {
            EntryBody::Feed(entry) => entry.updated(),
            EntryBody::Timeline(attrs) => attrs.updated_at,
        }
        .map(Into::into)
    }

    /// The time at which the entry published
    async fn published(&self) -> Option<scalar::Rfc3339Time> {
        match &self.body {
            EntryBody::Feed(entry) => entry.published(),
            EntryBody::Timeline(attrs) => attrs.published_at,
        }
        .map(Into::into)
    }

    /// Entry summary. When the feed declares none, the registry materializes
    /// a content-derived fallback at observation, so no fallback happens here.
    async fn summary(&self) -> Option<&str> {
        match &self.body {
            EntryBody::Feed(entry) => entry
                .summary()
                .map(types::Text::content)
                .or_else(|| entry.content().and_then(Content::body)),
            EntryBody::Timeline(attrs) => attrs.summary.as_deref(),
        }
    }

    /// Link to websiteurl at which this entry is published
    async fn website_url(&self) -> Option<&str> {
        match &self.body {
            EntryBody::Feed(entry) => entry.website_url(self.meta.feed.r#type()),
            EntryBody::Timeline(attrs) => attrs.website_url.as_deref(),
        }
    }
}

impl Entry {
    pub fn new(meta: Annotated<types::FeedMeta>, entry: SyndFeedEntry) -> Self {
        Self {
            id: entry.id().clone(),
            meta,
            body: EntryBody::Feed(Box::new(entry)),
        }
    }

    pub fn from_timeline_entry(node: TimelineEntry) -> Self {
        Self {
            id: node.entry_id,
            meta: node.feed_meta,
            body: EntryBody::Timeline(node.attrs),
        }
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
        self.0
            .feed
            .meta()
            .title()
            .map(synd_feed::types::Text::content)
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
        &'_ self,
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
            .map(move |entry| Entry::new(meta.clone(), entry.clone()))
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
                .iter()
                .enumerate()
                .map(|(idx, author)| Edge::new(idx, author.name().to_owned())),
        );

        c
    }

    /// Description of feed
    async fn description(&self) -> Option<&str> {
        self.0
            .feed
            .meta()
            .description()
            .map(synd_feed::types::Text::content)
    }

    async fn links(&self) -> Connection<usize, Link> {
        let mut c = Connection::new(false, false);
        c.edges.extend(
            self.0
                .feed
                .meta()
                .links()
                .iter()
                .map(Link::from)
                .enumerate()
                .map(|(idx, link)| Edge::new(idx, link)),
        );

        c
    }

    async fn website_url(&self) -> Option<&str> {
        self.0.feed.meta().website_url()
    }

    async fn generator(&self) -> Option<&str> {
        self.0
            .feed
            .meta()
            .generator()
            .map(synd_feed::types::Generator::content)
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
        self.0.feed.title().map(synd_feed::types::Text::content)
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
