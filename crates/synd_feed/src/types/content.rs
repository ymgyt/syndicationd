use bon::Builder;
use feed_rs::model as feedrs;
use serde::{Deserialize, Serialize};

use super::Link;

/// Content declared inline by an entry or referenced as an external resource.
#[allow(
    clippy::struct_field_names,
    reason = "content_type is the feed domain term shared with Text"
)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Builder)]
#[serde(rename_all = "snake_case")]
pub struct Content {
    body: Option<String>,
    content_type: String,
    length: Option<u64>,
    src: Option<Link>,
}

impl Content {
    /// Returns the inline content body when one was declared.
    pub fn body(&self) -> Option<&str> {
        self.body.as_deref()
    }

    /// Returns the declared media type, such as `text/plain` or `text/html`.
    pub fn content_type(&self) -> &str {
        &self.content_type
    }

    /// Returns the declared content length in bytes.
    pub fn length(&self) -> Option<u64> {
        self.length
    }

    /// Returns the external content resource when one was declared.
    pub fn src(&self) -> Option<&Link> {
        self.src.as_ref()
    }
}

impl From<feedrs::Content> for Content {
    fn from(value: feedrs::Content) -> Self {
        Self {
            body: value.body,
            content_type: value.content_type.to_string(),
            length: value.length,
            src: value.src.map(Into::into),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_feed_rs_content() {
        let source = feedrs::Content {
            body: Some("<p>Entry body</p>".into()),
            content_type: "text/html".parse().unwrap(),
            length: Some(17),
            src: Some(feedrs::Link {
                href: "https://example.com/entry".into(),
                rel: Some("alternate".into()),
                media_type: Some("text/html".into()),
                href_lang: Some("en".into()),
                title: Some("Entry".into()),
                length: Some(17),
            }),
        };

        let content = Content::from(source);

        assert_eq!(content.body(), Some("<p>Entry body</p>"));
        assert_eq!(content.content_type(), "text/html");
        assert_eq!(content.length(), Some(17));
        assert_eq!(
            content.src().map(Link::href),
            Some("https://example.com/entry")
        );
    }
}
