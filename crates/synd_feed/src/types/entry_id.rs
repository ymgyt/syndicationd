use std::{fmt, str::FromStr};

use feed_rs::model as feedrs;
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::{FeedType, FeedUrl, macros::impl_sqlx_encode_decode};

const PREFIX: &str = "synd:entry:v1:";
const DIGEST_HEX_LEN: usize = 64;
const DIGEST_INPUT_VERSION: &str = "synd-entry-id-input-v1";
pub(crate) const FEED_RS_MISSING_ID_MARKER: &str = "\x1Fsynd-entry-id-missing-v1\x1F";

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EntryIdError {
    #[error("entry id must start with {PREFIX}")]
    InvalidPrefix,
    #[error("entry id digest must be {expected} hex chars, got {actual}")]
    InvalidDigestLength { expected: usize, actual: usize },
    #[error("entry id digest must be lowercase hex")]
    InvalidDigest,
    #[error("entry id source field is missing")]
    MissingSourceField,
}

/// Opaque identifier assigned by synd to a feed entry.
///
/// The string representation is `synd:entry:v1:<64 lowercase hex>`.
/// It is deterministic for the same feed URL and source entry fields, while
/// deliberately hiding raw feed fields such as id, URL, title, content, and summary.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EntryId(String);

struct EntryIdInput<'a> {
    feed_url: &'a FeedUrl,
    feed_type: FeedType,
    entry: &'a feedrs::Entry,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct EntryIdSource<'a> {
    value: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EntryIdSourceSelector {
    NativeId,
    EntryUrl,
    Title,
    Content,
    Summary,
}

const SOURCE_PRIORITY: [EntryIdSourceSelector; 5] = [
    EntryIdSourceSelector::NativeId,
    EntryIdSourceSelector::EntryUrl,
    EntryIdSourceSelector::Title,
    EntryIdSourceSelector::Content,
    EntryIdSourceSelector::Summary,
];

impl EntryId {
    pub fn parse(value: impl Into<String>) -> Result<Self, EntryIdError> {
        let value = value.into();
        validate(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn from_feed_entry(
        feed_url: &FeedUrl,
        feed_type: FeedType,
        entry: &feedrs::Entry,
    ) -> Result<Self, EntryIdError> {
        let input = EntryIdInput {
            feed_url,
            feed_type,
            entry,
        };
        Self::from_input(&input)
    }

    fn from_input(input: &EntryIdInput<'_>) -> Result<Self, EntryIdError> {
        let source = input.source()?;
        Ok(Self::from_source(input.feed_url, source))
    }

    fn from_source(feed_url: &FeedUrl, source: EntryIdSource<'_>) -> Self {
        let mut hasher = Sha256::new();
        update_digest_field(&mut hasher, DIGEST_INPUT_VERSION);
        update_digest_field(&mut hasher, feed_url.as_str());
        update_digest_field(&mut hasher, source.as_str());
        let digest = hasher.finalize();

        let mut id = String::with_capacity(PREFIX.len() + DIGEST_HEX_LEN);
        id.push_str(PREFIX);
        for byte in digest {
            use std::fmt::Write as _;
            let _ = write!(id, "{byte:02x}");
        }

        Self(id)
    }
}

impl<'a> EntryIdInput<'a> {
    fn source(&self) -> Result<EntryIdSource<'a>, EntryIdError> {
        SOURCE_PRIORITY
            .iter()
            .find_map(|selector| selector.select(self))
            .ok_or(EntryIdError::MissingSourceField)
    }
}

impl EntryIdSourceSelector {
    fn select<'a>(self, input: &EntryIdInput<'a>) -> Option<EntryIdSource<'a>> {
        match self {
            Self::NativeId => EntryIdSource::from_native_id(input.entry),
            Self::EntryUrl => EntryIdSource::from_entry_url(input.feed_type, input.entry),
            Self::Title => EntryIdSource::from_title(input.entry),
            Self::Content => EntryIdSource::from_content(input.entry),
            Self::Summary => EntryIdSource::from_summary(input.entry),
        }
    }
}

impl<'a> EntryIdSource<'a> {
    fn as_str(self) -> &'a str {
        self.value
    }

    fn from_native_id(entry: &'a feedrs::Entry) -> Option<Self> {
        (entry.id != FEED_RS_MISSING_ID_MARKER).then(|| Self::from_text(entry.id.as_str()))?
    }

    fn from_entry_url(feed_type: FeedType, entry: &'a feedrs::Entry) -> Option<Self> {
        entry
            .links
            .iter()
            .find_map(|link| Self::from_entry_link(feed_type, link))
    }

    fn from_entry_link(feed_type: FeedType, link: &'a feedrs::Link) -> Option<Self> {
        let is_entry_url = match feed_type {
            FeedType::Atom => link.rel.as_deref() == Some("alternate"),
            FeedType::JSON => link.media_type.is_none(),
            FeedType::RSS0 | FeedType::RSS1 | FeedType::RSS2 => {
                link.rel.as_deref() != Some("self") && link.media_type.is_none()
            }
        };

        is_entry_url.then(|| Self::from_text(link.href.as_str()))?
    }

    fn from_title(entry: &'a feedrs::Entry) -> Option<Self> {
        entry
            .title
            .as_ref()
            .and_then(|title| Self::from_text(title.content.as_str()))
    }

    fn from_content(entry: &'a feedrs::Entry) -> Option<Self> {
        entry
            .content
            .as_ref()
            .and_then(|content| content.body.as_deref())
            .and_then(Self::from_text)
    }

    fn from_summary(entry: &'a feedrs::Entry) -> Option<Self> {
        entry
            .summary
            .as_ref()
            .and_then(|summary| Self::from_text(summary.content.as_str()))
    }

    fn from_text(value: &'a str) -> Option<Self> {
        let value = value.trim();
        (!value.is_empty()).then_some(Self { value })
    }
}

impl FromStr for EntryId {
    type Err = EntryIdError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::parse(value)
    }
}

impl TryFrom<&str> for EntryId {
    type Error = EntryIdError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::parse(value)
    }
}

impl fmt::Display for EntryId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl_sqlx_encode_decode!(EntryId as String);

pub(crate) fn feed_rs_missing_id_marker() -> String {
    FEED_RS_MISSING_ID_MARKER.to_owned()
}

fn validate(value: &str) -> Result<(), EntryIdError> {
    let Some(digest) = value.strip_prefix(PREFIX) else {
        return Err(EntryIdError::InvalidPrefix);
    };
    if digest.len() != DIGEST_HEX_LEN {
        return Err(EntryIdError::InvalidDigestLength {
            expected: DIGEST_HEX_LEN,
            actual: digest.len(),
        });
    }
    if !digest
        .bytes()
        .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        return Err(EntryIdError::InvalidDigest);
    }

    Ok(())
}

fn update_digest_field(hasher: &mut Sha256, field: &str) {
    hasher.update(field.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(field.as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_canonical_entry_id() {
        let id = EntryId::parse(format!("{}{}", PREFIX, "a".repeat(DIGEST_HEX_LEN))).unwrap();

        assert_eq!(
            id.as_str(),
            format!("{}{}", PREFIX, "a".repeat(DIGEST_HEX_LEN))
        );
    }

    #[test]
    fn parse_rejects_invalid_entry_id() {
        assert_eq!(
            EntryId::parse("raw-feed-id"),
            Err(EntryIdError::InvalidPrefix)
        );
        assert_eq!(
            EntryId::parse(format!("{PREFIX}abc")),
            Err(EntryIdError::InvalidDigestLength {
                expected: DIGEST_HEX_LEN,
                actual: 3,
            })
        );
        assert_eq!(
            EntryId::parse(format!("{}{}", PREFIX, "A".repeat(DIGEST_HEX_LEN))),
            Err(EntryIdError::InvalidDigest)
        );
    }

    #[test]
    fn source_field_uses_native_id_first() {
        let feed_url = FeedUrl::parse("https://example.com/feed.xml").unwrap();
        let entry = feedrs::Entry {
            id: "native-entry-id".to_owned(),
            ..Default::default()
        };

        assert_eq!(
            source_for(&feed_url, FeedType::Atom, &entry),
            Some("native-entry-id")
        );
    }

    #[test]
    fn source_field_uses_first_non_empty_rss_url_when_native_id_is_missing() {
        let feed_url = FeedUrl::parse("https://example.com/feed.xml").unwrap();
        let entry = feedrs::Entry {
            id: FEED_RS_MISSING_ID_MARKER.to_owned(),
            links: vec![
                link("  "),
                link(" https://example.com/entry "),
                link("https://example.com/ignored"),
            ],
            ..Default::default()
        };

        assert_eq!(
            source_for(&feed_url, FeedType::RSS2, &entry),
            Some("https://example.com/entry")
        );
    }

    #[test]
    fn source_field_uses_atom_alternate_url() {
        let feed_url = FeedUrl::parse("https://example.com/feed.xml").unwrap();
        let entry = feedrs::Entry {
            id: FEED_RS_MISSING_ID_MARKER.to_owned(),
            links: vec![
                link_with_rel("https://example.com/feed.xml", Some("self")),
                link_with_rel(" https://example.com/entry ", Some("alternate")),
            ],
            ..Default::default()
        };

        assert_eq!(
            source_for(&feed_url, FeedType::Atom, &entry),
            Some("https://example.com/entry")
        );
    }

    #[test]
    fn source_field_ignores_json_attachments_as_url() {
        let feed_url = FeedUrl::parse("https://example.com/feed.json").unwrap();
        let entry = feedrs::Entry {
            id: FEED_RS_MISSING_ID_MARKER.to_owned(),
            links: vec![feedrs::Link {
                href: "https://example.com/audio.mp3".to_owned(),
                rel: None,
                media_type: Some("audio/mpeg".to_owned()),
                href_lang: None,
                title: None,
                length: None,
            }],
            title: Some(text("Entry title")),
            ..Default::default()
        };

        assert_eq!(
            source_for(&feed_url, FeedType::JSON, &entry),
            Some("Entry title")
        );
    }

    #[test]
    fn source_field_uses_content_before_summary() {
        let feed_url = FeedUrl::parse("https://example.com/feed.xml").unwrap();
        let entry = feedrs::Entry {
            id: FEED_RS_MISSING_ID_MARKER.to_owned(),
            content: Some(feedrs::Content {
                body: Some(" Entry content ".to_owned()),
                ..Default::default()
            }),
            summary: Some(text("Entry summary")),
            ..Default::default()
        };

        assert_eq!(
            source_for(&feed_url, FeedType::Atom, &entry),
            Some("Entry content")
        );
    }

    #[test]
    fn source_field_uses_summary_when_content_is_empty() {
        let feed_url = FeedUrl::parse("https://example.com/feed.xml").unwrap();
        let entry = feedrs::Entry {
            id: FEED_RS_MISSING_ID_MARKER.to_owned(),
            content: Some(feedrs::Content {
                body: Some("  ".to_owned()),
                ..Default::default()
            }),
            summary: Some(text(" Entry summary ")),
            ..Default::default()
        };

        assert_eq!(
            source_for(&feed_url, FeedType::Atom, &entry),
            Some("Entry summary")
        );
    }

    #[test]
    fn from_feed_entry_rejects_entry_without_source_field() {
        let feed_url = FeedUrl::parse("https://example.com/feed.xml").unwrap();
        let entry = feedrs::Entry {
            id: FEED_RS_MISSING_ID_MARKER.to_owned(),
            ..Default::default()
        };

        assert_eq!(
            EntryId::from_feed_entry(&feed_url, FeedType::Atom, &entry),
            Err(EntryIdError::MissingSourceField)
        );
    }

    fn source_for<'a>(
        feed_url: &'a FeedUrl,
        feed_type: FeedType,
        entry: &'a feedrs::Entry,
    ) -> Option<&'a str> {
        EntryIdInput {
            feed_url,
            feed_type,
            entry,
        }
        .source()
        .ok()
        .map(EntryIdSource::as_str)
    }

    fn link(href: &str) -> feedrs::Link {
        link_with_rel(href, None)
    }

    fn link_with_rel(href: &str, rel: Option<&str>) -> feedrs::Link {
        feedrs::Link {
            href: href.to_owned(),
            rel: rel.map(ToOwned::to_owned),
            media_type: None,
            href_lang: None,
            title: None,
            length: None,
        }
    }

    fn text(content: &str) -> feedrs::Text {
        feedrs::Text {
            content_type: "text/plain".parse().unwrap(),
            src: None,
            content: content.to_owned(),
        }
    }
}
