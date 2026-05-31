use std::path::Path;

use synd_api::{
    cli::ServeOptions, dependency::Dependency, serve::auth::Authenticator, shutdown::Shutdown,
};
use synd_persistence::sqlite::{SqliteDatabase, SqliteFeedRegistryDb};
use synd_registry::{
    FeedRegistry, FeedRegistryConfig,
    event::{ApiEventPublisher, EventSubmitter, EventWakePublisher, WorkerSet},
    runtime::spawn_event_workers,
};

use crate::{Result, RuntimeDatabase};

/// Prepared synd-api dependency graph for one runtime database.
pub(crate) struct RuntimeApiService {
    dependency: Dependency,
    event_workers: WorkerSet,
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
        let config = FeedRegistryConfig::default();
        let journal = db.event_journal();
        let api_events = ApiEventPublisher::default();
        let wake_publisher = EventWakePublisher::new(config.event_wake_channel_capacity);

        let registry = {
            let event_submitter = { EventSubmitter::new(journal.clone(), wake_publisher.clone()) };

            FeedRegistry::with_api_events(db.clone(), config, api_events.clone(), event_submitter)
        };

        let event_workers = {
            spawn_event_workers(
                db,
                journal,
                wake_publisher,
                api_events,
                config,
                shutdown.cancellation_token(),
            )
        };

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
