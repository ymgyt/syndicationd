use thiserror::Error;

pub type StoreResult<T> = Result<T, StoreError>;

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl StoreError {
    pub fn internal(err: impl Into<anyhow::Error>) -> Self {
        Self::Internal(err.into())
    }
}

#[derive(Debug, Error)]
pub enum FeedRegistryError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    FeedProvider(#[from] crate::provider::FeedProviderError),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("feed is not subscribed: {0}")]
    NotSubscribed(synd_feed::types::FeedUrl),
    #[error("initial refresh failed: {0}")]
    InitialRefreshFailed(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}
