use chrono::DateTime;
use synd::types::Time;

use crate::client::{mutation, query};

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
pub struct FeedMeta {
    pub title: Option<String>,
    pub url: String,
    pub updated: Option<Time>,
    pub links: Vec<Link>,
    pub website_url: Option<String>,
}

impl From<query::subscription::FeedMeta> for FeedMeta {
    fn from(f: query::subscription::FeedMeta) -> Self {
        Self {
            title: f.title,
            url: f.url,
            updated: f.updated.map(parse_time),
            links: f.links.nodes.into_iter().map(From::from).collect(),
            website_url: f.website_url,
        }
    }
}

impl From<mutation::subscribe_feed::FeedMeta> for FeedMeta {
    fn from(f: mutation::subscribe_feed::FeedMeta) -> Self {
        Self {
            title: f.title,
            url: f.url,
            updated: f.updated.map(parse_time),
            links: f.links.nodes.into_iter().map(From::from).collect(),
            website_url: f.website_url,
        }
    }
}

fn parse_time(t: impl AsRef<str>) -> Time {
    DateTime::parse_from_rfc3339(t.as_ref())
        .expect("invalid rfc3339 time")
        .with_timezone(&chrono::Utc)
}
