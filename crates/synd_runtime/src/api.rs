use std::path::Path;

use synd_api::{
    dependency::Dependency,
    serve::{ServeOptions, auth::Authenticator},
    shutdown::Shutdown,
};
use synd_persistence::sqlite::{SqliteDatabase, SqliteFeedRegistryDb};
use synd_registry::{FeedRegistry, FeedRegistryConfig, event::WorkerSet};

use crate::{Result, RuntimeDatabase};

/// Prepared synd-api dependency graph for one runtime database.
pub(crate) struct ApiService {
    dependency: Dependency,
    event_workers: WorkerSet,
}

impl ApiService {
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
        let db = open_sqlite_registry_db(database_path).await?;
        let config = FeedRegistryConfig::default();
        let (registry, event_workers) =
            FeedRegistry::start(db, config, shutdown.cancellation_token());

        let dependency = Dependency::new(authenticator, registry, None, serve_options);

        Ok(Self {
            dependency,
            event_workers,
        })
    }

    pub(crate) fn into_parts(self) -> (Dependency, WorkerSet) {
        (self.dependency, self.event_workers)
    }
}

/// Opens and migrates the registry database as `SqliteFeedRegistryDb`.
async fn open_sqlite_registry_db(path: &Path) -> Result<SqliteFeedRegistryDb> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let db = SqliteDatabase::create_or_open(path).await?;
    db.migrate().await?;
    Ok(SqliteFeedRegistryDb::new(db))
}
