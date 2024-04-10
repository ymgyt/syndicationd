use std::{collections::HashMap, sync::Arc};

use kvsd::Value;
use serde::{Deserialize, Serialize};
use synd_feed::types::{self, Annotated, Category, Requirement};

use crate::repository::RepositoryError;

#[derive(Debug, Clone)]
pub struct Feed {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedSubscription {
    pub user_id: String,
    pub url: String,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct SubscribedFeeds {
    pub urls: Vec<String>,
    pub annotations: Option<HashMap<String, FeedAnnotations>>,
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

impl SubscribedFeeds {
    pub fn annotate<Iter>(self, feeds: Iter) -> impl Iterator<Item = Annotated<Arc<types::Feed>>>
    where
        Iter: IntoIterator<Item = Arc<types::Feed>>,
    {
        let mut annotations = self.annotations;

        feeds.into_iter().map(move |feed| {
            match annotations
                .as_mut()
                .and_then(|annotations| annotations.remove(feed.meta().url()))
            {
                Some(annotations) => Annotated {
                    feed,
                    requirement: annotations.requirement,
                    category: annotations.category,
                },
                None => Annotated {
                    feed,
                    requirement: None,
                    category: None,
                },
            }
        })
    }
}
