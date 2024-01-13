#[derive(Debug, Clone)]
pub struct Feed {
    url: String,
    #[allow(dead_code)]
    feed: feed_rs::model::Feed,
}

impl Feed {
    pub fn meta(&self) -> FeedMeta {
        FeedMeta::new(self.title().into(), self.url.clone())
    }

    pub fn title(&self) -> &str {
        self.feed
            .title
            .as_ref()
            .map(|text| text.content.as_str())
            .unwrap_or("???")
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

#[derive(Debug, Clone)]
pub struct FeedMeta {
    pub title: String,
    pub url: String,
}

impl FeedMeta {
    pub fn new(title: String, url: String) -> Self {
        Self { title, url }
    }
}
