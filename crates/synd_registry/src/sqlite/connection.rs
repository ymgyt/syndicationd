use std::{
    path::Path,
    time::{Duration, Instant},
};

use sqlx::{
    ConnectOptions as _, Sqlite, SqlitePool, Transaction,
    migrate::MigrateError as SqlxMigrateError,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use thiserror::Error;
use tracing::{debug, info};

use crate::RegistryDbError;

const SQLITE_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const SQLITE_MAX_CONNECTIONS: u32 = 1;

#[derive(Clone)]
pub struct SqliteDatabase {
    pool: SqlitePool,
}

/// Error returned while migrating a `SQLite` database.
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error(transparent)]
    Sqlx(#[from] SqlxMigrateError),
}

#[derive(Clone, Copy)]
enum FileMode {
    Existing,
    CreateIfMissing,
}

impl SqliteDatabase {
    /// Checks that an existing database has a readable SQLite schema.
    pub async fn check_readable(path: &Path) -> Result<(), RegistryDbError> {
        let mut connection = SqliteConnectOptions::new()
            .filename(path)
            .read_only(true)
            .connect()
            .await
            .map_err(RegistryDbError::retryable)?;
        sqlx::query("PRAGMA schema_version")
            .fetch_one(&mut connection)
            .await
            .map_err(RegistryDbError::retryable)?;
        Ok(())
    }

    pub async fn open(db_path: impl AsRef<Path>) -> Result<Self, RegistryDbError> {
        Self::open_file(db_path, FileMode::Existing).await
    }

    pub async fn create_or_open(db_path: impl AsRef<Path>) -> Result<Self, RegistryDbError> {
        Self::open_file(db_path, FileMode::CreateIfMissing).await
    }

    pub async fn migrate(&self) -> Result<(), MigrationError> {
        let migrator = sqlx::migrate!("./migrations");
        let schema_version = migrator
            .migrations
            .last()
            .map_or(0, |migration| migration.version);
        let started_at = Instant::now();
        debug!(schema_version, "Running SQLite migrations");
        migrator.run(&self.pool).await?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn checking_readability_preserves_the_database_and_rejects_invalid_files()
    -> anyhow::Result<()> {
        use sqlx::Connection as _;

        let dir = tempfile::tempdir()?;
        let path = dir.path().join("example.db");
        let mut connection = SqliteConnectOptions::new()
            .filename(&path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Delete)
            .connect()
            .await?;
        sqlx::query("CREATE TABLE example (id INTEGER)")
            .execute(&mut connection)
            .await?;
        connection.close().await?;

        let original = std::fs::read(&path)?;
        SqliteDatabase::check_readable(&path).await?;
        assert_eq!(std::fs::read(&path)?, original);

        let invalid = dir.path().join("invalid.db");
        std::fs::write(&invalid, b"not a SQLite database")?;
        assert!(SqliteDatabase::check_readable(&invalid).await.is_err());
        Ok(())
    }

    #[tokio::test]
    async fn named_timelines_preserve_the_old_membership_and_sync_position() -> anyhow::Result<()> {
        let db = SqliteDatabase::in_memory().await?;
        sqlx::migrate!("./migrations").run_to(5, &db.pool).await?;
        sqlx::raw_sql(
            r"
            INSERT INTO feed (pk, url) VALUES (1, 'https://example.com/feed');
            INSERT INTO feed_subscription (subscriber_id, feed_pk, crawl_policy_json, subscribed_at)
                VALUES ('without-timeline', 1, '{}', '2026-10-01');
            INSERT INTO entry (entry_id, feed_pk, entry_json, order_time)
                VALUES ('a', 1, '{}', '2026-10-01'), ('b', 1, '{}', '2026-10-02');
            INSERT INTO timeline (subscriber_id, last_seq) VALUES ('reader', 12), ('empty', 0);
            INSERT INTO timeline_entry (subscriber_id, entry_id, order_time, seq, deleted)
                VALUES ('reader', 'a', '2026-10-01', 4, 0), ('reader', 'b', '2026-10-02', 9, 1);
            ",
        )
        .execute(&db.pool)
        .await?;

        db.migrate().await?;
        let timelines: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT subscriber_id, name, last_seq FROM timeline ORDER BY subscriber_id",
        )
        .fetch_all(&db.pool)
        .await?;
        assert_eq!(
            timelines,
            [
                ("empty".into(), "all".into(), 0),
                ("reader".into(), "all".into(), 12)
            ]
        );
        let reader_id: Uuid =
            sqlx::query_scalar("SELECT timeline_id FROM timeline WHERE subscriber_id = 'reader'")
                .fetch_one(&db.pool)
                .await?;
        assert_eq!(reader_id.get_version_num(), 4);
        let entries: Vec<(String, String, i64, bool)> = sqlx::query_as(
            "SELECT entry_id, CAST(order_time AS TEXT), seq, deleted FROM timeline_entry WHERE timeline_id = ? ORDER BY seq",
        )
        .bind(reader_id)
        .fetch_all(&db.pool)
        .await?;
        assert_eq!(
            entries,
            [
                ("a".into(), "2026-10-01".into(), 4, false),
                ("b".into(), "2026-10-02".into(), 9, true),
            ]
        );
        Ok(())
    }
}
