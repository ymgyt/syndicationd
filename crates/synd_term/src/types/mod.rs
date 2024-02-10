use chrono::DateTime;

use crate::client::{mutation, query};

mod time;
pub use time::{Time, TimeExt};

mod page_info;
pub use page_info::PageInfo;

#[derive(Debug)]
pub struct Link {
    pub href: String,
    pub rel: Option<String>,
    pub media_type: Option<String>,
    pub title: Option<String>,
}

impl From<query::subscription::Link> for Link {
    fn from(v: query::subscription::Link) -> Self {
        Self {
            href: v.href,
            rel: v.rel,
            media_type: v.media_type,
            title: v.title,
        }
    }
}

impl From<mutation::subscribe_feed::Link> for Link {
    fn from(v: mutation::subscribe_feed::Link) -> Self {
        Self {
            href: v.href,
            rel: v.rel,
            media_type: v.media_type,
            title: v.title,
        }
    }
}

#[derive(Debug)]
pub struct EntryMeta {
    pub title: Option<String>,
    pub published: Option<Time>,
    pub summary: Option<String>,
}

impl From<query::subscription::EntryMeta> for EntryMeta {
    fn from(e: query::subscription::EntryMeta) -> Self {
        Self {
            title: e.title,
            published: e.published.map(parse_time),
            summary: e.summary,
        }
    }
}

impl From<mutation::subscribe_feed::EntryMeta> for EntryMeta {
    fn from(e: mutation::subscribe_feed::EntryMeta) -> Self {
        Self {
            title: e.title,
            published: e.published.map(parse_time),
            summary: e.summary,
        }
    }
}

impl EntryMeta {
    pub fn summary_text(&self, width: usize) -> Option<String> {
        self.summary
            .as_deref()
            .map(|summary| html2text::from_read(summary.as_bytes(), width))
    }
}

#[derive(Debug)]
pub struct Feed {
    pub title: Option<String>,
    pub url: String,
    pub updated: Option<Time>,
    pub links: Vec<Link>,
    pub website_url: Option<String>,
    pub description: Option<String>,
    pub entries: Vec<EntryMeta>,
}

impl From<query::subscription::Feed> for Feed {
    fn from(f: query::subscription::Feed) -> Self {
        Self {
            title: f.title,
            url: f.url,
            updated: f.updated.map(parse_time),
            links: f.links.nodes.into_iter().map(From::from).collect(),
            website_url: f.website_url,
            description: f.description,
            entries: f.entries.nodes.into_iter().map(From::from).collect(),
        }
    }
}

impl From<mutation::subscribe_feed::Feed> for Feed {
    fn from(f: mutation::subscribe_feed::Feed) -> Self {
        Self {
            title: f.title,
            url: f.url,
            updated: f.updated.map(parse_time),
            links: f.links.nodes.into_iter().map(From::from).collect(),
            website_url: f.website_url,
            description: f.description,
            entries: f.entries.nodes.into_iter().map(From::from).collect(),
        }
    }
}

#[derive(Debug)]
pub struct Entry {
    pub title: Option<String>,
    pub published: Option<Time>,
    pub website_url: Option<String>,
    pub summary: Option<String>,
    pub feed_title: Option<String>,
    pub feed_url: String,
}

impl Entry {
    pub fn summary_text(&self, width: usize) -> Option<String> {
        self.summary
            .as_deref()
            .map(|summary| html2text::from_read(summary.as_bytes(), width))
    }
}

impl From<query::entries::Entry> for Entry {
    fn from(v: query::entries::Entry) -> Self {
        Self {
            title: v.title,
            published: v.published.map(parse_time),
            website_url: v.website_url,
            feed_title: v.feed.title,
            feed_url: v.feed.url,
            summary: v.summary,
        }
    }
}

fn parse_time(t: impl AsRef<str>) -> Time {
    DateTime::parse_from_rfc3339(t.as_ref())
        .expect("invalid rfc3339 time")
        .with_timezone(&chrono::Utc)
}
