use thiserror::Error;

pub type RegistryDbResult<T> = Result<T, RegistryDbError>;

#[derive(Debug, Error)]
pub enum RegistryDbError {
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl RegistryDbError {
    pub fn internal(err: impl Into<anyhow::Error>) -> Self {
        Self::Internal(err.into())
    }
}

#[derive(Debug, Error)]
pub enum FeedRegistryError {
    #[error(transparent)]
    Db(#[from] RegistryDbError),
    #[error(transparent)]
    FeedProvider(#[from] crate::legacy::FeedProviderError),
    #[error(transparent)]
    EventRuntime(#[from] crate::event::EventRuntimeError),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("feed is not subscribed: {0}")]
    NotSubscribed(synd_feed::types::FeedUrl),
    #[error("initial refresh failed: {0}")]
    InitialRefreshFailed(String),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}
