#[derive(Debug, Clone)]
pub struct Feed {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FeedSubscription {
    pub user_id: String,
    pub url: String,
}
