#[derive(Debug)]
pub struct FeedMeta {
    url: String,
    title: String,
}

impl FeedMeta {
    pub fn new(title: String, url: String) -> Self {
        Self { url, title }
    }

    pub fn url(&self) -> &str {
        self.url.as_str()
    }

    pub fn title(&self) -> &str {
        self.title.as_str()
    }
}
