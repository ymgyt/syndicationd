use std::{path::Path, time::Duration};

use sqlx::{
    Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use synd_registry::RegistryDbError;
use tracing::info;

#[derive(Clone)]
pub struct SqliteDatabase {
    pool: SqlitePool,
}

#[derive(Clone, Copy)]
enum FileMode {
    Existing,
    CreateIfMissing,
}

impl SqliteDatabase {
    pub async fn open(db_path: impl AsRef<Path>) -> Result<Self, RegistryDbError> {
        Self::open_file(db_path, FileMode::Existing).await
    }

    pub async fn create_or_open(db_path: impl AsRef<Path>) -> Result<Self, RegistryDbError> {
        Self::open_file(db_path, FileMode::CreateIfMissing).await
    }

    pub async fn migrate(&self) -> Result<(), RegistryDbError> {
        info!("Run persistence migrations...");
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(RegistryDbError::permanent)
    }

    pub async fn begin(&self) -> Result<Transaction<'_, Sqlite>, RegistryDbError> {
        self.pool.begin().await.map_err(RegistryDbError::retryable)
    }

    async fn open_file(db_path: impl AsRef<Path>, mode: FileMode) -> Result<Self, RegistryDbError> {
        let opts = Self::file_options(db_path, mode);
        Self::build_pool(opts).await
    }

    fn file_options(db_path: impl AsRef<Path>, mode: FileMode) -> SqliteConnectOptions {
        Self::common_options(
            SqliteConnectOptions::new()
                .filename(db_path)
                .create_if_missing(matches!(mode, FileMode::CreateIfMissing))
                .journal_mode(SqliteJournalMode::Wal),
        )
    }

    fn common_options(opts: SqliteConnectOptions) -> SqliteConnectOptions {
        opts.foreign_keys(true).busy_timeout(Duration::from_secs(5))
    }

    async fn build_pool(opts: SqliteConnectOptions) -> Result<Self, RegistryDbError> {
        info!(?opts, "Connecting to sqlite...");
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .map_err(RegistryDbError::retryable)?;

        Ok(Self { pool })
    }

    #[cfg(test)]
    pub async fn in_memory() -> Result<Self, RegistryDbError> {
        Self::build_pool(Self::common_options(
            SqliteConnectOptions::new().in_memory(true),
        ))
        .await
    }
}
