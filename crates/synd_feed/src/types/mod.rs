use std::{borrow::Cow, fmt::Display};

use chrono::{DateTime, Utc};
use feed_rs::model::{self as feedrs, Generator, Link, Person, Text};

pub type Time = DateTime<Utc>;

mod requirement;
pub use requirement::Requirement;

mod category;
pub use category::Category;

mod url;
pub use url::FeedUrl;

mod feed_type;
pub use feed_type::FeedType;

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct EntryId<'a>(Cow<'a, str>);

impl<'a, T> From<T> for EntryId<'a>
where
    T: Into<Cow<'a, str>>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

impl Display for EntryId<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0.as_ref())
    }
}

#[derive(Debug, Clone)]
pub struct Entry(feedrs::Entry);

impl Entry {
    pub fn id(&self) -> EntryId<'static> {
        EntryId(Cow::Owned(self.0.id.clone()))
    }

    pub fn id_ref(&self) -> EntryId<'_> {
        EntryId(Cow::Borrowed(self.0.id.as_str()))
    }

    pub fn title(&self) -> Option<&str> {
        self.0.title.as_ref().map(|text| text.content.as_str())
    }

    pub fn updated(&self) -> Option<Time> {
        self.0.updated
    }

    pub fn published(&self) -> Option<Time> {
        self.0.published
    }

    pub fn summary(&self) -> Option<&str> {
        self.0.summary.as_ref().map(|text| text.content.as_str())
    }

    pub fn content(&self) -> Option<&str> {
        self.0
            .content
            .as_ref()
            .and_then(|content| content.body.as_deref())
    }

    pub fn website_url(&self, feed_type: FeedType) -> Option<&str> {
        link::find_website_url(feed_type, &self.0.links)
    }

    /// Return approximate entry bytes size
    pub fn approximate_size(&self) -> usize {
        let content_size = self
            .0
            .content
            .as_ref()
            .and_then(|content| content.body.as_deref())
            .map_or(0, str::len);

        let summary_size = self
            .0
            .summary
            .as_ref()
            .map_or(0, |summary| summary.content.len());

        content_size + summary_size
    }
}

#[derive(Debug, Clone)]
pub struct FeedMeta {
    url: FeedUrl,
    // feed_rs models
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
    pub fn r#type(&self) -> FeedType {
        self.feed_type
    }

    pub fn url(&self) -> &FeedUrl {
        &self.url
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_ref().map(|text| text.content.as_str())
    }

    pub fn updated(&self) -> Option<Time> {
        self.updated.or(self.published)
    }

    pub fn authors(&self) -> impl Iterator<Item = &str> {
        self.authors.iter().map(|person| person.name.as_str())
    }

    pub fn description(&self) -> Option<&str> {
        self.description.as_ref().map(|text| text.content.as_str())
    }

    pub fn links(&self) -> impl Iterator<Item = &feedrs::Link> {
        self.links.iter()
    }

    /// Return website link to which feed syndicate
    pub fn website_url(&self) -> Option<&str> {
        link::find_website_url(self.r#type(), &self.links)
    }

    pub fn generator(&self) -> Option<&str> {
        self.generator.as_ref().map(|g| g.content.as_str())
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
    pub fn parts(self) -> (FeedMeta, Vec<Entry>) {
        (self.meta, self.entries)
    }

    pub fn meta(&self) -> &FeedMeta {
        &self.meta
    }

    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    /// Return approximate Feed byte size
    pub fn approximate_size(&self) -> usize {
        self.entries().map(Entry::approximate_size).sum()
    }
}

impl From<(FeedUrl, feed_rs::model::Feed)> for Feed {
    fn from((url, feed): (FeedUrl, feedrs::Feed)) -> Self {
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
        let meta = FeedMeta {
            url,
            feed_type: feed_type.into(),
            title,
            updated,
            authors,
            description,
            links,
            generator,
            published,
        };
        let entries = entries.into_iter().map(Entry).collect();

        Feed { meta, entries }
    }
}

mod link {
    use feed_rs::model::Link;

    use crate::types::FeedType;

    pub fn find_website_url<'a>(
        feed_type: FeedType,
        links: impl IntoIterator<Item = &'a Link>,
    ) -> Option<&'a str> {
        let mut links = links.into_iter();
        match feed_type {
            // Find rel == alternate link
            FeedType::Atom => links
                .find(|link| link.rel.as_deref() == Some("alternate"))
                .map(|link| link.href.as_str()),

            // how to detect homepage(website) url?
            // ignore .json extension link
            FeedType::JSON => links
                .find(|link| {
                    !std::path::Path::new(link.href.as_str())
                        .extension()
                        .map_or(false, |ext| ext.eq_ignore_ascii_case("json"))
                })
                .map(|link| link.href.as_str()),

            FeedType::RSS0 => {
                tracing::warn!("RSS0 is used! {:?}", links.collect::<Vec<_>>());
                None
            }

            // Use the first link whose rel is not "self"
            FeedType::RSS1 | FeedType::RSS2 => links
                .find(|link| link.rel.as_deref() != Some("self"))
                .map(|link| link.href.as_str()),
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn rss_ignore_rel_self() {
            let links = vec![
                Link {
                    href: "https://syndicationd.ymgyt.io/".into(),
                    title: None,
                    rel: None,
                    media_type: None,
                    href_lang: None,
                    length: None,
                },
                Link {
                    href: "https://syndicationd.ymgyt.io/atom.xml".into(),
                    title: None,
                    rel: Some("self".into()),
                    media_type: None,
                    href_lang: None,
                    length: None,
                },
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
                Link {
                    href: "https://syndicationd.ymgyt.io/atom.xml".into(),
                    title: None,
                    rel: Some("self".into()),
                    media_type: None,
                    href_lang: None,
                    length: None,
                },
                Link {
                    href: "https://syndicationd.ymgyt.io/".into(),
                    title: None,
                    rel: Some("alternate".into()),
                    media_type: None,
                    href_lang: None,
                    length: None,
                },
            ];

            assert_eq!(
                find_website_url(FeedType::Atom, &links),
                Some("https://syndicationd.ymgyt.io/")
            );
        }

        #[test]
        fn json_ignore_json_ext() {
            let links = vec![
                Link {
                    href: "https://kubernetes.io/docs/reference/issues-security/official-cve-feed/index.json".into(),
                    title: None,
                    rel: None,
                    media_type: None,
                    href_lang: None,
                    length: None,
                },
                Link {
                    href: "https://kubernetes.io".into(),
                    title: None,
                    rel: None,
                    media_type: None,
                    href_lang: None,
                    length: None,
                },
            ];

            assert_eq!(
                find_website_url(FeedType::JSON, &links),
                Some("https://kubernetes.io")
            );
        }
    }
}
