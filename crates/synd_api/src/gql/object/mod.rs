use std::{collections::BTreeMap, sync::Arc};

use async_graphql::{
    ID, Json, Object, SimpleObject,
    connection::{Connection, ConnectionNameType, Edge, EdgeNameType, EmptyFields},
};
use serde_json::Value;
use synd_feed::{
    entry::{self, Entry as SyndFeedEntry},
    types::{self, FeedType, FeedUrl, SubscriptionLabels},
};
use synd_registry::query::SubscribedEntry;
use synd_support::html::HtmlConverter;

use crate::gql::scalar;

use self::{excerpt::EntryExcerpt, id::FeedIdV1};

mod excerpt;
pub mod id;

#[derive(SimpleObject)]
pub(crate) struct Link {
    pub href: String,
    pub rel: Option<String>,
    pub media_type: Option<String>,
    pub href_lang: Option<String>,
    pub title: Option<String>,
    pub length: Option<u64>,
}

impl From<&types::Link> for Link {
    fn from(value: &types::Link) -> Self {
        Self {
            href: value.href().to_owned(),
            rel: value.rel().map(ToOwned::to_owned),
            media_type: value.media_type().map(ToOwned::to_owned),
            href_lang: value.href_lang().map(ToOwned::to_owned),
            title: value.title().map(ToOwned::to_owned),
            length: value.length(),
        }
    }
}

#[derive(SimpleObject)]
struct Person<'a> {
    name: Option<&'a str>,
    uri: Option<&'a str>,
    email: Option<&'a str>,
}

impl<'a> From<&'a types::Person> for Person<'a> {
    fn from(person: &'a types::Person) -> Self {
        Self {
            name: person.name(),
            uri: person.uri(),
            email: person.email(),
        }
    }
}

/// Text and metadata supplied by the feed for a title or summary.
struct Text<'a>(&'a types::Text);

#[Object]
impl Text<'_> {
    async fn content(&self) -> &str {
        self.0.content()
    }

    async fn content_type(&self) -> &str {
        self.0.content_type()
    }

    async fn src(&self) -> Option<&str> {
        self.0.src()
    }
}

/// Inline or externally referenced entry content declared by the feed.
struct Content<'a>(&'a entry::Content);

#[Object]
impl Content<'_> {
    async fn body(&self) -> Option<&str> {
        self.0.body()
    }

    async fn content_type(&self) -> &str {
        self.0.content_type()
    }

    async fn length(&self) -> Option<u64> {
        self.0.length()
    }

    async fn src(&self) -> Option<Link> {
        self.0.src().map(Link::from)
    }
}

pub(crate) struct Entry {
    feed: Arc<FeedMeta>,
    entry: SyndFeedEntry,
}

const DEFAULT_EXCERPT_WIDTH: usize = 200;

#[Object]
impl Entry {
    /// Entry id
    async fn id(&self) -> ID {
        self.entry.id().as_str().into()
    }

    /// Feed of this entry
    async fn feed(&self) -> &FeedMeta {
        &self.feed
    }
    /// Entry title
    async fn title(&self) -> Option<&str> {
        self.entry.title().map(types::Text::content)
    }

    /// Entry title with its declared media type and source.
    async fn title_text(&self) -> Option<Text<'_>> {
        self.entry.title().map(Text)
    }

    async fn authors(&self) -> Vec<Person<'_>> {
        self.entry.authors().iter().map(Person::from).collect()
    }

    async fn links(&self) -> Vec<Link> {
        self.entry.links().iter().map(Link::from).collect()
    }

    async fn labels(&self) -> Json<&BTreeMap<String, Value>> {
        Json(self.entry.labels())
    }

    /// Summary supplied by the feed.
    async fn feed_summary(&self) -> Option<Text<'_>> {
        self.entry.summary().map(Text)
    }

    async fn content(&self) -> Option<Content<'_>> {
        self.entry.content().map(Content)
    }

    /// Time at which the entry was last modified
    async fn updated(&self) -> Option<scalar::Rfc3339Time> {
        self.entry.updated().map(Into::into)
    }

    /// The time at which the entry published
    async fn published(&self) -> Option<scalar::Rfc3339Time> {
        self.entry.published().map(Into::into)
    }

    /// Plain-text excerpt from the feed summary or, if absent, the inline content.
    #[expect(
        clippy::unused_async,
        reason = "async-graphql requires an async resolver"
    )]
    async fn excerpt(
        &self,
        cx: &async_graphql::Context<'_>,
        #[graphql(
            desc = "Maximum Unicode display width of the entire excerpt, from 0 to 1000.",
            default_with = "Some(DEFAULT_EXCERPT_WIDTH)",
            validator(maximum = 1000)
        )]
        max_unicode_width: Option<usize>,
    ) -> async_graphql::Result<Option<EntryExcerpt>> {
        let converter = cx.data::<Arc<dyn HtmlConverter>>()?;
        EntryExcerpt::from_entry(
            &self.entry,
            max_unicode_width.unwrap_or(DEFAULT_EXCERPT_WIDTH),
            converter.as_ref(),
        )
        .map_err(async_graphql::Error::new_with_source)
    }

    /// Link to websiteurl at which this entry is published
    async fn website_url(&self) -> Option<&str> {
        self.entry.website_url(self.feed.meta.r#type())
    }
}

impl From<SubscribedEntry> for Entry {
    fn from(node: SubscribedEntry) -> Self {
        Self {
            feed: Arc::new(FeedMeta {
                meta: node.feed_meta,
                subscription_labels: node.subscription_labels,
            }),
            entry: node.entry,
        }
    }
}

pub(super) struct Feed<'a> {
    pub(super) feed: &'a types::Feed,
    pub(super) subscription_labels: &'a SubscriptionLabels,
}

type FeedEntriesConnection =
    Connection<usize, Entry, EmptyFields, EmptyFields, FeedEntryConnectionName, FeedEntryEdgeName>;

/// One overfetched first page of a feed's entries.
struct FeedEntryPage {
    entries: Vec<Entry>,
    has_next_page: bool,
}

impl FeedEntryPage {
    fn read(feed: &Feed<'_>, first: Option<i32>) -> Self {
        let limit = usize::try_from(first.unwrap_or(5).max(0)).unwrap_or_default();
        let feed_meta = Arc::new(FeedMeta {
            meta: feed.feed.meta().clone(),
            subscription_labels: feed.subscription_labels.clone(),
        });
        Self::from_overfetch(
            feed.feed
                .entries()
                .take(limit.saturating_add(1))
                .map(|entry| Entry {
                    feed: Arc::clone(&feed_meta),
                    entry: entry.clone(),
                })
                .collect(),
            limit,
        )
    }

    fn from_overfetch(mut entries: Vec<Entry>, limit: usize) -> Self {
        let has_next_page = entries.len() > limit;
        entries.truncate(limit);
        Self {
            entries,
            has_next_page,
        }
    }
}

impl From<FeedEntryPage> for FeedEntriesConnection {
    fn from(page: FeedEntryPage) -> Self {
        let mut connection = Self::new(false, page.has_next_page);
        connection.edges.extend(
            page.entries
                .into_iter()
                .enumerate()
                .map(|(index, entry)| Edge::new(index, entry)),
        );
        connection
    }
}

#[Object]
impl Feed<'_> {
    /// Feed Id
    async fn id(&self) -> ID {
        FeedIdV1::new(self.feed.meta().url()).into()
    }

    /// Undering feed specification
    async fn r#type(&self) -> FeedType {
        self.feed.meta().r#type()
    }

    /// Feed title
    async fn title(&self) -> Option<&str> {
        self.feed
            .meta()
            .title()
            .map(synd_feed::types::Text::content)
    }

    /// Feed URL
    async fn url(&self) -> &FeedUrl {
        self.feed.meta().url()
    }

    /// The time at which the feed was last modified
    async fn updated(&self) -> Option<scalar::Rfc3339Time> {
        self.feed.meta().updated().map(Into::into)
    }

    /// Feed entries
    async fn entries(
        &'_ self,
        #[graphql(default = 5)] first: Option<i32>,
    ) -> FeedEntriesConnection {
        FeedEntryPage::read(self, first).into()
    }

    /// Feed author names
    async fn authors(&self) -> Connection<usize, String> {
        let mut c = Connection::new(false, false);
        c.edges.extend(
            self.feed
                .meta()
                .authors()
                .iter()
                .enumerate()
                .filter_map(|(idx, author)| {
                    author.name().map(|name| Edge::new(idx, name.to_owned()))
                }),
        );

        c
    }

    /// Description of feed
    async fn description(&self) -> Option<&str> {
        self.feed
            .meta()
            .description()
            .map(synd_feed::types::Text::content)
    }

    async fn links(&self) -> Connection<usize, Link> {
        let mut c = Connection::new(false, false);
        c.edges.extend(
            self.feed
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
        self.feed.meta().website_url()
    }

    async fn generator(&self) -> Option<&str> {
        self.feed
            .meta()
            .generator()
            .map(synd_feed::types::Generator::content)
    }

    /// Labels this subscriber assigned to the feed.
    async fn labels(&self) -> Json<&SubscriptionLabels> {
        Json(self.subscription_labels)
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

struct FeedMeta {
    meta: types::FeedMeta,
    subscription_labels: SubscriptionLabels,
}

#[Object]
impl FeedMeta {
    /// Title of the feed
    async fn title(&self) -> Option<&str> {
        self.meta.title().map(types::Text::content)
    }

    /// Url of the feed
    async fn url(&self) -> &FeedUrl {
        self.meta.url()
    }

    /// Labels this subscriber assigned to the feed.
    async fn labels(&self) -> Json<&SubscriptionLabels> {
        Json(&self.subscription_labels)
    }
}

#[cfg(test)]
mod tests {
    use async_graphql::{EmptyMutation, EmptySubscription, Schema, value};
    use synd_feed::{
        feed::service::FeedService,
        types::{FeedUrl, SubscriptionLabels},
    };
    use synd_registry::query::SubscribedEntry;

    use super::Entry;

    #[tokio::test]
    async fn external_content_keeps_its_reference_and_has_no_inline_body() {
        let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom"><entry>
                <id>urn:entry:external</id><title>Example</title><summary>Feed summary</summary>
                <content type="text/html" src="https://example.com/body"/>
            </entry></feed>"#;
        let url = FeedUrl::parse("https://example.com/feed").unwrap();
        let feed = FeedService::parse_feed(url, xml.as_bytes()).unwrap();
        let entry = Entry::from(SubscribedEntry {
            entry: feed.entries().next().unwrap().clone(),
            feed_meta: feed.meta().clone(),
            subscription_labels: SubscriptionLabels::default(),
        });
        let response = Schema::build(entry, EmptyMutation, EmptySubscription)
            .finish()
            .execute(
                "{
                    title
                    titleText { content contentType src }
                    feedSummary { content contentType src }
                    content { body contentType length src { href } }
                }",
            )
            .await
            .into_result()
            .unwrap();
        assert_eq!(
            response.data,
            value!({
                "title": "Example",
                "titleText": { "content": "Example", "contentType": "text/plain", "src": null },
                "feedSummary": { "content": "Feed summary", "contentType": "text/plain", "src": null },
                "content": {
                    "body": null,
                    "contentType": "text/html",
                    "length": null,
                    "src": { "href": "https://example.com/body" }
                }
            })
        );
    }
}
