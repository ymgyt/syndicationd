use std::borrow::Cow;

use bon::Builder;
use chrono::{DateTime, Utc};
use feed_rs::model as feedrs;
use serde::{Deserialize, Serialize};
use tracing::warn;

pub type Time = DateTime<Utc>;

mod requirement;
pub use requirement::Requirement;

mod category;
pub use category::Category;

mod url;
pub use url::FeedUrl;

mod feed_type;
pub use feed_type::FeedType;

mod entry_id;
pub(crate) use entry_id::feed_rs_missing_id_marker;
pub use entry_id::{EntryId, EntryIdError};

mod content;
pub use content::Content;

mod entry;
pub use entry::Entry;

mod macros;

/// Text content with its media type and optional source URI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Builder)]
#[serde(rename_all = "snake_case")]
pub struct Text {
    content: String,
    content_type: String,
    src: Option<String>,
}

impl Text {
    /// Returns the textual payload.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the declared media type, such as `text/plain` or `text/html`.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Returns the source URI when the text references external content.
    pub fn src(&self) -> Option<&str> {
        self.src.as_deref()
    }
}

impl From<feedrs::Text> for Text {
    fn from(value: feedrs::Text) -> Self {
        Self {
            content: value.content,
            content_type: value.content_type.to_string(),
            src: value.src,
        }
    }
}

/// Person credited by a feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Builder)]
#[serde(rename_all = "snake_case")]
pub struct Person {
    name: String,
    uri: Option<String>,
    email: Option<String>,
}

impl Person {
    /// Returns the human-readable person name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the associated profile or homepage URI.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }

    /// Returns the email address when supplied by the feed.
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }
}

impl From<feedrs::Person> for Person {
    fn from(value: feedrs::Person) -> Self {
        Self {
            name: value.name,
            uri: value.uri,
            email: value.email,
        }
    }
}

/// Link relation declared by a feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Builder)]
#[serde(rename_all = "snake_case")]
pub struct Link {
    href: String,
    rel: Option<String>,
    media_type: Option<String>,
    href_lang: Option<String>,
    title: Option<String>,
    length: Option<u64>,
}

impl Link {
    /// Returns the target URI.
    pub fn href(&self) -> &str {
        &self.href
    }

    /// Returns the link relation, such as `self` or `alternate`.
    pub fn rel(&self) -> Option<&str> {
        self.rel.as_deref()
    }

    /// Returns the media type of the target resource.
    pub fn media_type(&self) -> Option<&str> {
        self.media_type.as_deref()
    }

    /// Returns the language tag for the target resource.
    pub fn href_lang(&self) -> Option<&str> {
        self.href_lang.as_deref()
    }

    /// Returns the human-readable link title.
    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    /// Returns the target resource length in bytes.
    pub fn length(&self) -> Option<u64> {
        self.length
    }
}

impl From<feedrs::Link> for Link {
    fn from(value: feedrs::Link) -> Self {
        Self {
            href: value.href,
            rel: value.rel,
            media_type: value.media_type,
            href_lang: value.href_lang,
            title: value.title,
            length: value.length,
        }
    }
}

/// Software metadata declared by a feed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Generator {
    content: String,
    uri: Option<String>,
    version: Option<String>,
}

impl Generator {
    /// Returns the generator name or description.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Returns the generator URI.
    pub fn uri(&self) -> Option<&str> {
        self.uri.as_deref()
    }

    /// Returns the generator version.
    pub fn version(&self) -> Option<&str> {
        self.version.as_deref()
    }
}

impl From<feedrs::Generator> for Generator {
    fn from(value: feedrs::Generator) -> Self {
        Self {
            content: value.content,
            uri: value.uri,
            version: value.version,
        }
    }
}

/// Metadata declared at the feed level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FeedMeta {
    url: FeedUrl,
    feed_type: FeedType,
    title: Option<Text>,
    updated: Option<Time>,
    authors: Vec<Person>,
    description: Option<Text>,
    links: Vec<Link>,
    generator: Option<Generator>,
    published: Option<Time>,
}

#[derive(Debug, Clone)]
pub struct Annotated<T> {
    pub feed: T,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}

impl<T> Annotated<T> {
    pub fn project<U>(&self, f: impl Fn(&T) -> U) -> Annotated<U> {
        Annotated {
            feed: f(&self.feed),
            requirement: self.requirement,
            category: self.category.clone(),
        }
    }
}

impl<T> Annotated<T> {
    pub fn new(feed: T) -> Self {
        Self {
            feed,
            requirement: None,
            category: None,
        }
    }
}

impl FeedMeta {
    /// Returns the feed format.
    pub fn r#type(&self) -> FeedType {
        self.feed_type
    }

    /// Returns the URL from which this feed was read.
    pub fn url(&self) -> &FeedUrl {
        &self.url
    }

    /// Returns the feed title.
    pub fn title(&self) -> Option<&Text> {
        self.title.as_ref()
    }

    /// Returns the feed update time, falling back to published time.
    pub fn updated(&self) -> Option<Time> {
        self.updated.or(self.published)
    }

    /// Returns people credited as feed authors.
    pub fn authors(&self) -> &[Person] {
        &self.authors
    }

    /// Returns the feed description.
    pub fn description(&self) -> Option<&Text> {
        self.description.as_ref()
    }

    /// Returns link relations declared by the feed.
    pub fn links(&self) -> &[Link] {
        &self.links
    }

    /// Returns the website URL represented by the feed links.
    pub fn website_url(&self) -> Option<&str> {
        link::find_website_url(self.r#type(), &self.links)
    }

    /// Returns software metadata declared by the feed.
    pub fn generator(&self) -> Option<&Generator> {
        self.generator.as_ref()
    }
}

impl<'a> From<&'a FeedMeta> for Cow<'a, FeedMeta> {
    fn from(value: &'a FeedMeta) -> Self {
        Cow::Borrowed(value)
    }
}

impl From<FeedMeta> for Cow<'static, FeedMeta> {
    fn from(value: FeedMeta) -> Self {
        Cow::Owned(value)
    }
}

#[derive(Debug, Clone)]
pub struct Feed {
    meta: FeedMeta,
    entries: Vec<Entry>,
}

impl Feed {
    pub(crate) fn from_feed_rs(url: FeedUrl, feed: feedrs::Feed) -> Self {
        let feed_rs::model::Feed {
            feed_type,
            title,
            updated,
            authors,
            description,
            links,
            generator,
            published,
            entries,
            ..
        } = feed;
        let feed_type = feed_type.into();
        let entries = entries
            .into_iter()
            .filter_map(|entry| match Entry::from_feed_rs(&url, feed_type, entry) {
                Ok(entry) => Some(entry),
                Err(err) => {
                    warn!(
                        error = %err,
                        feed_url = url.as_str(),
                        "skip feed entry because EntryId cannot be generated"
                    );
                    None
                }
            })
            .collect();
        let meta = FeedMeta {
            url,
            feed_type,
            title: title.map(Into::into),
            updated,
            authors: authors.into_iter().map(Into::into).collect(),
            description: description.map(Into::into),
            links: links.into_iter().map(Into::into).collect(),
            generator: generator.map(Into::into),
            published,
        };
        Feed { meta, entries }
    }

    pub fn parts(self) -> (FeedMeta, Vec<Entry>) {
        (self.meta, self.entries)
    }

    pub fn meta(&self) -> &FeedMeta {
        &self.meta
    }

    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }
}

mod link {
    use tracing::warn;

    use crate::types::{FeedType, Link};

    pub fn find_website_url<'a>(
        feed_type: FeedType,
        links: impl IntoIterator<Item = &'a Link>,
    ) -> Option<&'a str> {
        let mut links = links.into_iter();
        match feed_type {
            // Find rel == alternate link
            FeedType::Atom => links
                .find(|link| link.rel() == Some("alternate"))
                .map(Link::href),

            // how to detect homepage(website) url?
            // ignore .json extension link
            FeedType::JSON => links
                .find(|link| {
                    !std::path::Path::new(link.href())
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                })
                .map(Link::href),

            FeedType::RSS0 => {
                warn!("RSS0 is used! {:?}", links.collect::<Vec<_>>());
                None
            }

            // Use the first link whose rel is not "self"
            FeedType::RSS1 | FeedType::RSS2 => links
                .find(|link| link.rel() != Some("self"))
                .map(Link::href),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn rss_ignore_rel_self() {
            let links = vec![
                Link::builder()
                    .href("https://syndicationd.ymgyt.io/".into())
                    .build(),
                Link::builder()
                    .href("https://syndicationd.ymgyt.io/atom.xml".into())
                    .rel("self".into())
                    .build(),
            ];

            assert_eq!(
                find_website_url(FeedType::RSS1, &links),
                Some("https://syndicationd.ymgyt.io/")
            );
            assert_eq!(
                find_website_url(FeedType::RSS2, &links),
                Some("https://syndicationd.ymgyt.io/")
            );
        }

        #[test]
        fn atom_use_rel_alternate() {
            let links = vec![
                Link::builder()
                    .href("https://syndicationd.ymgyt.io/atom.xml".into())
                    .rel("self".into())
                    .build(),
                Link::builder()
                    .href("https://syndicationd.ymgyt.io/".into())
                    .rel("alternate".into())
                    .build(),
            ];

            assert_eq!(
                find_website_url(FeedType::Atom, &links),
                Some("https://syndicationd.ymgyt.io/")
            );
        }

        #[test]
        fn json_ignore_json_ext() {
            let links = vec![
                Link::builder()
                    .href("https://kubernetes.io/docs/reference/issues-security/official-cve-feed/index.json".into())
                    .build(),
                Link::builder()
                    .href("https://kubernetes.io".into())
                    .build(),
            ];

            assert_eq!(
                find_website_url(FeedType::JSON, &links),
                Some("https://kubernetes.io")
            );
        }
    }
}
