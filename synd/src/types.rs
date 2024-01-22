use chrono::{DateTime, Utc};
use feed_rs::model as feedrs;

pub use feedrs::FeedType;

pub type Time = DateTime<Utc>;

#[derive(Debug, Clone)]
pub struct Feed {
    url: String,
    #[allow(dead_code)]
    feed: feedrs::Feed,
}

impl Feed {
    pub fn r#type(&self) -> FeedType {
        self.feed.feed_type.clone()
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
        match self.feed.feed_type {
            // find rel == alternate link
            FeedType::Atom => self
                .feed
                .links
                .iter()
                .find(|link| link.rel.as_deref() == Some("alternate"))
                .map(|link| link.href.as_str()),

            // TODO
            FeedType::JSON => todo!(),

            // TODO
            FeedType::RSS0 => todo!(),

            // use first one
            FeedType::RSS1 | FeedType::RSS2 => {
                assert!(self.feed.links.len() < 2);
                self.feed.links.first().map(|link| link.href.as_str())
            }
        }
    }
}

impl From<(String, feed_rs::model::Feed)> for Feed {
    fn from(feed: (String, feed_rs::model::Feed)) -> Self {
        Feed {
            url: feed.0,
            feed: feed.1,
        }
    }
}
