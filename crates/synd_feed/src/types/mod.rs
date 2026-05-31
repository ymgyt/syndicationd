use std::borrow::Cow;

use chrono::{DateTime, Utc};
use feed_rs::model::{self as feedrs, Generator, Link, Person, Text};
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

mod entry;
pub use entry::Entry;

mod macros;

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
            title,
            updated,
            authors,
            description,
            links,
            generator,
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

    /// Return approximate Feed byte size
    pub fn approximate_size(&self) -> usize {
        self.entries().map(Entry::approximate_size).sum()
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
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
                })
                .map(|link| link.href.as_str()),

            FeedType::RSS0 => {
                warn!("RSS0 is used! {:?}", links.collect::<Vec<_>>());
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
