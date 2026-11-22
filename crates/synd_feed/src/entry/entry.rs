use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use bon::Builder;
use feed_rs::model as feedrs;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::types::{FeedType, FeedUrl, Link, Person, Text, Time, link};

use super::{Content, EntryId, EntryIdError};

pub use feed_rs::model::Entry as SourceEntry;

/// A category declared by the publisher of an entry.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SourceCategory {
    pub term: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

impl SourceCategory {
    pub const LABEL_KEY: &str = "source.categories";
}

impl From<feedrs::Category> for SourceCategory {
    fn from(category: feedrs::Category) -> Self {
        Self {
            term: category.term,
            scheme: category.scheme,
            label: category.label,
        }
    }
}

#[derive(Debug, Error)]
#[error("invalid entry label {key}: {source}")]
pub struct LabelDecodeError {
    key: &'static str,
    source: serde_json::Error,
}

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
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    #[builder(default)]
    labels: BTreeMap<String, Value>,
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

    pub fn labels(&self) -> &BTreeMap<String, Value> {
        &self.labels
    }

    pub fn source_categories(&self) -> Result<Option<Vec<SourceCategory>>, LabelDecodeError> {
        self.labels
            .get(SourceCategory::LABEL_KEY)
            .map(Vec::deserialize)
            .transpose()
            .map_err(|source| LabelDecodeError {
                key: SourceCategory::LABEL_KEY,
                source,
            })
    }

    /// Stores categories in a stable order for entry comparison and persistence.
    pub fn set_source_categories(&mut self, categories: impl IntoIterator<Item = SourceCategory>) {
        let categories: BTreeSet<_> = categories
            .into_iter()
            .filter(|category| !category.term.is_empty())
            .collect();
        if categories.is_empty() {
            self.labels.remove(SourceCategory::LABEL_KEY);
        } else {
            self.labels.insert(
                SourceCategory::LABEL_KEY.to_owned(),
                serde_json::to_value(categories).expect("source categories contain only strings"),
            );
        }
    }

    pub fn website_url(&self, feed_type: FeedType) -> Option<&str> {
        link::find_website_url(feed_type, &self.links)
    }
}

/// An entry with its stable identity resolved from the original source fields.
pub struct IdentifiedEntry {
    id: EntryId,
    format: FeedType,
    source: SourceEntry,
}

impl IdentifiedEntry {
    pub fn identify(
        url: &FeedUrl,
        format: FeedType,
        source: SourceEntry,
    ) -> Result<Self, EntryIdError> {
        let id = EntryId::from_feed_entry(url, format, &source)?;
        Ok(Self { id, format, source })
    }

    pub fn into_entry(self) -> Entry {
        let feedrs::Entry {
            title,
            updated,
            authors,
            content,
            links,
            summary,
            published,
            categories,
            ..
        } = self.source;
        let mut entry = Entry {
            id: self.id,
            title: title.map(Into::into),
            updated,
            authors: authors.into_iter().map(Into::into).collect(),
            content: content.map(Into::into),
            links: links.into_iter().map(Into::into).collect(),
            summary: summary.map(Into::into),
            published,
            labels: BTreeMap::new(),
        };
        if self.format != FeedType::JSON {
            entry.set_source_categories(categories.into_iter().map(SourceCategory::from));
        }
        entry
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
                name: Some("Author".into()),
                uri: Some("https://example.com/author".into()),
                email: Some("author@example.com".into()),
                role: None,
            }],
            content: Some(feedrs::Content {
                body: Some("Entry body".into()),
                content_type: "text/plain".parse().unwrap(),
                length: Some(10),
                src: None,
            }),
            links: vec![feedrs::Link {
                href: "https://example.com/entry".into(),
                target: None,
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

        let entry = IdentifiedEntry::identify(
            &FeedUrl::parse("https://example.com/feed.xml").unwrap(),
            FeedType::Atom,
            source,
        )
        .unwrap()
        .into_entry();

        assert_eq!(entry.title().map(Text::content), Some("Entry title"));
        assert_eq!(entry.updated(), Some(updated));
        assert_eq!(entry.authors()[0].name(), Some("Author"));
        assert_eq!(entry.content().and_then(Content::body), Some("Entry body"));
        assert_eq!(entry.links()[0].href(), "https://example.com/entry");
        assert_eq!(entry.summary().map(Text::content), Some("Entry summary"));
        assert_eq!(entry.published(), Some(published));
    }

    #[test]
    fn atom_entry_categories_keep_their_terms_schemes_and_labels() {
        let url = FeedUrl::parse("https://example.com/feed").unwrap();
        let xml = r#"<feed xmlns="http://www.w3.org/2005/Atom">
            <title>Research</title><category term="feed only"/>
            <entry><id>urn:example:1</id><title>Paper</title>
                <category term="AI, ML" scheme="https://example.com/topics" label="Research"/>
            </entry></feed>"#;
        let feed = crate::feed::service::FeedService::parse_feed(url, xml.as_bytes()).unwrap();
        let entry = feed.entries().next().unwrap();
        assert_eq!(
            entry.labels()["source.categories"],
            serde_json::json!([{ "term": "AI, ML", "scheme": "https://example.com/topics", "label": "Research" }])
        );
    }

    #[test]
    fn labels_are_optional_and_decoded_only_on_typed_access() {
        let mut document = serde_json::json!({
            "id": format!("synd:entry:v1:{}", "a".repeat(64)),
            "authors": [], "links": []
        });
        let legacy: Entry = serde_json::from_value(document.clone()).unwrap();
        assert!(legacy.labels().is_empty());
        assert!(legacy.source_categories().unwrap().is_none());

        document["labels"] = serde_json::json!({
            "source.categories": [{"term": 123}],
            "custom.note": {"text": "read me"}
        });
        let entry: Entry = serde_json::from_value(document.clone()).unwrap();
        assert_eq!(entry.labels()["custom.note"]["text"], "read me");
        assert!(entry.source_categories().is_err());
        assert_eq!(
            serde_json::to_value(entry).unwrap()["labels"],
            document["labels"]
        );
    }
}
