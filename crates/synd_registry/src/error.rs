use std::error::Error as StdError;

use thiserror::Error;

/// Result type returned by registry storage adapters.
pub type RegistryDbResult<T> = Result<T, RegistryDbError>;

/// Error raised by registry storage operations.
#[derive(Debug, Error)]
pub enum RegistryDbError {
    #[error(transparent)]
    Retryable(Box<dyn StdError + Send + Sync>),
    #[error(transparent)]
    Permanent(Box<dyn StdError + Send + Sync>),
    #[error("registry invariant violated: {0}")]
    Invariant(String),
}

impl RegistryDbError {
    pub fn retryable<E>(err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Retryable(Box::new(err))
    }

    pub fn permanent<E>(err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Permanent(Box::new(err))
    }

    pub fn invariant(message: impl Into<String>) -> Self {
        Self::Invariant(message.into())
    }
}

/// Error returned by registry command and query operations.
#[derive(Debug, Error)]
pub enum FeedRegistryError {
    #[error(transparent)]
    Db(#[from] RegistryDbError),
    #[error(transparent)]
    Rejected(#[from] crate::subscription::SubReject),
    #[error(transparent)]
    CrawlRequestRejected(#[from] crate::crawl::request::CrawlRequestReject),
}
