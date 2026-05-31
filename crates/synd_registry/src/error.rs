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
    EventSubmitter(#[from] crate::event::EventSubmitterError),
}
