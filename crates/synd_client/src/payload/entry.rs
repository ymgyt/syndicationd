use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use synd_feed::{
    entry::{Content, EntryId},
    types::{FeedUrl, Link, Person, Text, Time},
};

use super::FeedMeta;

/// A stored entry with the content and metadata supplied by its feed.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub struct EntryDetails {
    #[cfg_attr(feature = "jsonschema", schemars(with = "String"))]
    pub id: EntryId,
    pub title: Option<Text>,
    /// Entry URL on the publisher's website.
    pub url: Option<String>,
    pub feed: Feed,
    pub published: Option<Time>,
    pub updated: Option<Time>,
    pub authors: Vec<Person>,
    pub links: Vec<Link>,
    pub labels: BTreeMap<String, Value>,
    /// Original feed summary.
    pub summary: Option<Text>,
    pub content: Option<Content>,
}

/// Source feed of an entry.
#[derive(Debug, Deserialize, Serialize)]
#[cfg_attr(feature = "jsonschema", derive(schemars::JsonSchema))]
pub struct Feed {
    pub title: Option<String>,
    /// URL used to subscribe to the feed.
    pub url: FeedUrl,
}

impl From<FeedMeta> for Feed {
    fn from(feed: FeedMeta) -> Self {
        Self {
            title: feed.title,
            url: feed.url,
        }
    }
}
