use std::{
    path::Path,
    time::{Duration, Instant},
};

use sqlx::{
    Sqlite, SqlitePool, Transaction,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use synd_registry::RegistryDbError;
use tracing::{debug, info};

const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const SQLITE_MAX_CONNECTIONS: u32 = 1;

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
        let migrator = sqlx::migrate!("./migrations");
        let schema_version = migrator
            .migrations
            .last()
            .map_or(0, |migration| migration.version);
        let started_at = Instant::now();
        debug!(schema_version, "Running SQLite migrations");
        migrator
            .run(&self.pool)
            .await
            .map_err(RegistryDbError::permanent)?;
        info!(
            schema_version,
            duration_ms = started_at.elapsed().as_millis(),
            "SQLite database ready"
        );

        Ok(())
    }

    pub async fn begin(&self) -> Result<Transaction<'_, Sqlite>, RegistryDbError> {
        self.pool.begin().await.map_err(RegistryDbError::retryable)
    }

    async fn open_file(db_path: impl AsRef<Path>, mode: FileMode) -> Result<Self, RegistryDbError> {
        let db_path = db_path.as_ref();
        debug!(
            database = %db_path.display(),
            read_only = false,
            create_if_missing = matches!(mode, FileMode::CreateIfMissing),
            journal_mode = "wal",
            foreign_keys = true,
            busy_timeout_ms = SQLITE_BUSY_TIMEOUT.as_millis(),
            max_connections = SQLITE_MAX_CONNECTIONS,
            "Connecting to SQLite"
        );
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
        opts.foreign_keys(true).busy_timeout(SQLITE_BUSY_TIMEOUT)
    }

    async fn build_pool(opts: SqliteConnectOptions) -> Result<Self, RegistryDbError> {
        let database = opts.get_filename().to_owned();
        let pool = SqlitePoolOptions::new()
            .max_connections(SQLITE_MAX_CONNECTIONS)
            .connect_with(opts)
            .await
            .map_err(RegistryDbError::retryable)?;
        debug!(database = %database.display(), "Connected to SQLite");

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
