use synd_registry::{RegistryDbError, RegistryDbResult, event::EventEncodingError};

use crate::compression::CompressionError;

pub(crate) type SqliteResult<T> = Result<T, SqliteError>;

/// Error type used inside the `SQLite` registry adapter before crossing the port boundary.
#[derive(Debug, thiserror::Error)]
pub(crate) enum SqliteError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Event(#[from] EventEncodingError),
    #[error(transparent)]
    Compression(#[from] CompressionError),
    #[error("decode: {0}")]
    DecodeMessage(String),
    #[error("{entity} not found: {key}")]
    NotFound { entity: &'static str, key: String },
}

impl SqliteError {
    pub(crate) fn decode_message(message: impl Into<String>) -> Self {
        Self::DecodeMessage(message.into())
    }

    pub(crate) fn not_found(entity: &'static str, key: impl Into<String>) -> Self {
        Self::NotFound {
            entity,
            key: key.into(),
        }
    }
}

pub(crate) trait DecodeResultExt<T> {
    fn decode(self) -> SqliteResult<T>;
}

impl<T, E> DecodeResultExt<T> for Result<T, E>
where
    E: std::fmt::Display + Send + Sync + 'static,
{
    fn decode(self) -> SqliteResult<T> {
        self.map_err(|err| SqliteError::decode_message(err.to_string()))
    }
}

pub(crate) trait IntoDbResult<T> {
    fn db(self) -> RegistryDbResult<T>;
}

impl<T> IntoDbResult<T> for SqliteResult<T> {
    fn db(self) -> RegistryDbResult<T> {
        self.map_err(RegistryDbError::internal)
    }
}
