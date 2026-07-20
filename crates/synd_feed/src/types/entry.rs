use std::fmt;

use bon::Builder;
use feed_rs::model as feedrs;
use serde::{Deserialize, Serialize};

use super::{Content, EntryId, EntryIdError, FeedType, FeedUrl, Link, Person, Text, Time, link};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Builder)]
#[serde(rename_all = "snake_case")]
pub struct Entry {
    id: EntryId,
    title: Option<Text>,
    updated: Option<Time>,
    #[builder(default)]
    authors: Vec<Person>,
    content: Option<Content>,
    #[builder(default)]
    links: Vec<Link>,
    summary: Option<Text>,
    published: Option<Time>,
}

impl Entry {
    pub fn id(&self) -> &EntryId {
        &self.id
    }

    pub fn title(&self) -> Option<&Text> {
        self.title.as_ref()
    }

    pub fn updated(&self) -> Option<Time> {
        self.updated
    }

    pub fn authors(&self) -> &[Person] {
        &self.authors
    }

    pub fn content(&self) -> Option<&Content> {
        self.content.as_ref()
    }

    pub fn links(&self) -> &[Link] {
        &self.links
    }

    pub fn published(&self) -> Option<Time> {
        self.published
    }

    pub fn summary(&self) -> Option<&Text> {
        self.summary.as_ref()
    }

    pub fn website_url(&self, feed_type: FeedType) -> Option<&str> {
        link::find_website_url(feed_type, &self.links)
    }

    pub(super) fn from_feed_rs(
        feed_url: &FeedUrl,
        feed_type: FeedType,
        entry: feedrs::Entry,
    ) -> Result<Self, EntryIdError> {
        let id = EntryId::from_feed_entry(feed_url, feed_type, &entry)?;
        let feedrs::Entry {
            title,
            updated,
            authors,
            content,
            links,
            summary,
            published,
            ..
        } = entry;
        Ok(Self {
            id,
            title: title.map(Into::into),
            updated,
            authors: authors.into_iter().map(Into::into).collect(),
            content: content.map(Into::into),
            links: links.into_iter().map(Into::into).collect(),
            summary: summary.map(Into::into),
            published,
        })
    }
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Entry")
            .field("id", &self.id)
            .field("title", &self.title)
            .field("updated", &self.updated())
            .field("published", &self.published())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_owned_fields_from_feed_rs_entry() {
        let updated = "2026-07-19T12:00:00Z".parse().unwrap();
        let published = "2026-07-18T12:00:00Z".parse().unwrap();
        let source = feedrs::Entry {
            id: "source-entry-id".into(),
            title: Some(feedrs::Text {
                content: "Entry title".into(),
                content_type: "text/plain".parse().unwrap(),
                src: None,
            }),
            updated: Some(updated),
            authors: vec![feedrs::Person {
                name: "Author".into(),
                uri: Some("https://example.com/author".into()),
                email: Some("author@example.com".into()),
            }],
            content: Some(feedrs::Content {
                body: Some("Entry body".into()),
                content_type: "text/plain".parse().unwrap(),
                length: Some(10),
                src: None,
            }),
            links: vec![feedrs::Link {
                href: "https://example.com/entry".into(),
                rel: Some("alternate".into()),
                media_type: Some("text/html".into()),
                href_lang: None,
                title: None,
                length: None,
            }],
            summary: Some(feedrs::Text {
                content: "Entry summary".into(),
                content_type: "text/plain".parse().unwrap(),
                src: None,
            }),
            published: Some(published),
            ..Default::default()
        };

        let entry = Entry::from_feed_rs(
            &FeedUrl::parse("https://example.com/feed.xml").unwrap(),
            FeedType::Atom,
            source,
        )
        .unwrap();

        assert_eq!(entry.title().map(Text::content), Some("Entry title"));
        assert_eq!(entry.updated(), Some(updated));
        assert_eq!(entry.authors()[0].name(), "Author");
        assert_eq!(entry.content().and_then(Content::body), Some("Entry body"));
        assert_eq!(entry.links()[0].href(), "https://example.com/entry");
        assert_eq!(entry.summary().map(Text::content), Some("Entry summary"));
        assert_eq!(entry.published(), Some(published));
    }
}
