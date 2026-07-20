use synd_feed::{entry::EntryId, types::FeedUrl};
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
    #[error("stored entry id mismatch: relational={relational}, document={document}")]
    EntryIdMismatch {
        relational: EntryId,
        document: EntryId,
    },
    #[error("stored feed URL mismatch: relational={relational}, document={document}")]
    FeedUrlMismatch {
        relational: Box<FeedUrl>,
        document: Box<FeedUrl>,
    },
    #[error("entry write affected {rows_affected} rows: feed_pk={feed_pk}, entry_id={entry_id}")]
    EntryWriteCount {
        feed_pk: i64,
        entry_id: EntryId,
        rows_affected: u64,
    },
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

    fn is_retryable(&self) -> bool {
        match self {
            Self::Sqlx(err) => sqlx_error_is_retryable(err),
            Self::Json(_)
            | Self::Event(_)
            | Self::Compression(_)
            | Self::DecodeMessage(_)
            | Self::NotFound { .. }
            | Self::EntryIdMismatch { .. }
            | Self::FeedUrlMismatch { .. }
            | Self::EntryWriteCount { .. } => false,
        }
    }
}

fn sqlx_error_is_retryable(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(err) => err
            .code()
            .and_then(|code| code.parse::<i32>().ok())
            .is_some_and(sqlite_error_code_is_retryable),
        sqlx::Error::Io(_)
        | sqlx::Error::Tls(_)
        | sqlx::Error::PoolTimedOut
        | sqlx::Error::WorkerCrashed
        | sqlx::Error::BeginFailed => true,
        _ => false,
    }
}

fn sqlite_error_code_is_retryable(extended_code: i32) -> bool {
    const SQLITE_BUSY: i32 = 5;
    const SQLITE_LOCKED: i32 = 6;
    const SQLITE_IOERR: i32 = 10;

    matches!(
        extended_code & 0xff,
        SQLITE_BUSY | SQLITE_LOCKED | SQLITE_IOERR
    )
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
        self.map_err(|err| {
            if err.is_retryable() {
                RegistryDbError::retryable(err)
            } else {
                RegistryDbError::permanent(err)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::*;

    #[test]
    fn io_error_is_retryable() {
        let result: SqliteResult<()> = Err(SqliteError::Sqlx(sqlx::Error::Io(io::Error::new(
            io::ErrorKind::ConnectionReset,
            "connection reset",
        ))));

        let err = result.db().unwrap_err();

        assert!(matches!(err, RegistryDbError::Retryable(_)));
    }

    #[test]
    fn decode_error_is_permanent() {
        let result: SqliteResult<()> = Err(SqliteError::decode_message("invalid entry id"));

        let err = result.db().unwrap_err();

        assert!(matches!(err, RegistryDbError::Permanent(_)));
    }

    #[tokio::test]
    async fn constraint_violation_is_permanent() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query("CREATE TABLE item (id INTEGER PRIMARY KEY)")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO item (id) VALUES (1)")
            .execute(&pool)
            .await
            .unwrap();
        let result: SqliteResult<()> = sqlx::query("INSERT INTO item (id) VALUES (1)")
            .execute(&pool)
            .await
            .map(|_| ())
            .map_err(SqliteError::from);

        let err = result.db().unwrap_err();

        assert!(matches!(err, RegistryDbError::Permanent(_)));
    }
}
