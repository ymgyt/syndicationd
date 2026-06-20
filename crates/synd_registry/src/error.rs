use std::error::Error as StdError;

use thiserror::Error;

/// Result type returned by registry storage adapters.
pub type RegistryDbResult<T> = Result<T, RegistryDbError>;

/// Error raised by registry storage operations.
#[derive(Debug, Error)]
pub enum RegistryDbError {
    #[error(transparent)]
    Internal(#[from] Box<dyn StdError + Send + Sync>),
    #[error("internal registry error: {0}")]
    InternalMessage(String),
}

impl RegistryDbError {
    pub fn internal<E>(err: E) -> Self
    where
        E: StdError + Send + Sync + 'static,
    {
        Self::Internal(Box::new(err))
    }

    pub fn internal_message(message: impl Into<String>) -> Self {
        Self::InternalMessage(message.into())
    }
}

/// Error returned by registry command and query operations.
#[derive(Debug, Error)]
pub enum FeedRegistryError {
    #[error(transparent)]
    Db(#[from] RegistryDbError),
    #[error(transparent)]
    Rejected(#[from] crate::subscription::SubReject),
}
