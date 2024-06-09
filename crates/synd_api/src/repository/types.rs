use std::collections::HashMap;

use kvsd::Value;
use serde::{Deserialize, Serialize};
use synd_feed::types::{Category, FeedUrl, Requirement};

use crate::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct Feed {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSubscription {
    pub user_id: String,
    pub url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SubscribedFeeds {
    pub urls: Vec<FeedUrl>,
    pub annotations: Option<HashMap<FeedUrl, FeedAnnotations>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FeedAnnotations {
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}

impl TryFrom<Value> for SubscribedFeeds {
    type Error = RepositoryError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        serde_json::from_slice(&value).map_err(RepositoryError::internal)
    }
}

impl TryFrom<SubscribedFeeds> for Value {
    type Error = RepositoryError;

    fn try_from(value: SubscribedFeeds) -> Result<Self, Self::Error> {
        let value = serde_json::to_vec(&value).map_err(RepositoryError::internal)?;
        Ok(Value::new(value).unwrap())
    }
}
