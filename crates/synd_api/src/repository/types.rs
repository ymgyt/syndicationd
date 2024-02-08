use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Feed {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSubscription {
    pub user_id: String,
    pub url: String,
}
