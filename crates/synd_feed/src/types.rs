use std::{borrow::Cow, fmt::Display};

use chrono::{DateTime, Utc};
use feed_rs::model as feedrs;

pub use feedrs::FeedType;

pub type Time = DateTime<Utc>;
pub type FeedUrl = String;

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

impl<'a> Display for EntryId<'a> {
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

    pub fn website_url(&self, feed_type: &FeedType) -> Option<&str> {
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
    // TODO: extrace feedrs data
    // no entries
    feed: feedrs::Feed,
}

impl FeedMeta {
    pub fn r#type(&self) -> &FeedType {
        &self.feed.feed_type
    }

    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    pub fn title(&self) -> Option<&str> {
        self.feed.title.as_ref().map(|text| text.content.as_str())
    }

    pub fn updated(&self) -> Option<Time> {
        self.feed.updated
    }

    pub fn authors(&self) -> impl Iterator<Item = &str> {
        self.feed.authors.iter().map(|person| person.name.as_str())
    }

    pub fn description(&self) -> Option<&str> {
        self.feed
            .description
            .as_ref()
            .map(|text| text.content.as_str())
    }

    pub fn links(&self) -> impl Iterator<Item = &feedrs::Link> {
        self.feed.links.iter()
    }

    /// Return website link to which feed syndicate
    pub fn website_url(&self) -> Option<&str> {
        link::find_website_url(self.r#type(), &self.feed.links)
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
    fn from((url, mut feed): (FeedUrl, feedrs::Feed)) -> Self {
        let entries = std::mem::take(&mut feed.entries);
        let entries = entries.into_iter().map(Entry).collect();

        let meta = FeedMeta { url, feed };
        Feed { meta, entries }
    }
}

mod link {
    use feed_rs::model::{FeedType, Link};

    pub fn find_website_url<'a>(
        feed_type: &FeedType,
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

            // TODO
            FeedType::RSS0 => todo!(),

            // Use the first link whose rel is not "self"
            FeedType::RSS1 | FeedType::RSS2 => links
                .find(|link| link.rel.as_deref() != Some("self"))
                .map(|link| link.href.as_str()),
        }
    }

    #[cfg(test)]
    mod test {
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
                find_website_url(&FeedType::RSS1, &links),
                Some("https://syndicationd.ymgyt.io/")
            );
            assert_eq!(
                find_website_url(&FeedType::RSS2, &links),
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
                find_website_url(&FeedType::Atom, &links),
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
                find_website_url(&FeedType::JSON, &links),
                Some("https://kubernetes.io")
            );
        }
    }
}
