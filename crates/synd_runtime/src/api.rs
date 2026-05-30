use std::path::Path;

use synd_api::{
    cli::ServeOptions, dependency::Dependency, serve::auth::Authenticator, shutdown::Shutdown,
};
use synd_persistence::sqlite::{SqliteDatabase, SqliteEventJournal, SqliteFeedRegistryDb};
use synd_registry::{FeedRegistryConfig, FeedRegistryRuntime};

use crate::{Result, RuntimeDatabase};

type ApiRegistryRuntime = FeedRegistryRuntime<SqliteFeedRegistryDb, SqliteEventJournal>;

/// Prepared synd-api dependency graph for one runtime database.
pub(crate) struct RuntimeApiService {
    dependency: Dependency,
    registry_runtime: ApiRegistryRuntime,
}

impl RuntimeApiService {
    pub(crate) async fn from_database(
        database: &RuntimeDatabase,
        authenticator: Authenticator,
        serve_options: ServeOptions,
        shutdown: &Shutdown,
    ) -> Result<Self> {
        Self::from_database_path(
            database.sqlite_path(),
            authenticator,
            serve_options,
            shutdown,
        )
        .await
    }

    pub(crate) async fn from_database_path(
        database_path: &Path,
        authenticator: Authenticator,
        serve_options: ServeOptions,
        shutdown: &Shutdown,
    ) -> Result<Self> {
        let db = RuntimeRepository::open(database_path).await?;
        let registry_runtime = FeedRegistryRuntime::start(
            db.clone(),
            db.event_journal(),
            FeedRegistryConfig::default(),
            shutdown.cancellation_token(),
        );
        registry_runtime.reconcile_startup().await;
        let dependency = Dependency::new(
            authenticator,
            registry_runtime.registry(),
            None,
            serve_options,
        );

        Ok(Self {
            dependency,
            registry_runtime,
        })
    }

    pub(crate) fn into_parts(self) -> (Dependency, ApiRegistryRuntime) {
        (self.dependency, self.registry_runtime)
    }
}

/// Opens and migrates the `SQLite` repository used by a runtime API.
struct RuntimeRepository;

impl RuntimeRepository {
    async fn open(path: &Path) -> Result<SqliteFeedRegistryDb> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let db = SqliteDatabase::create_or_open(path).await?;
        db.migrate().await?;
        Ok(SqliteFeedRegistryDb::new(db))
    }
}
