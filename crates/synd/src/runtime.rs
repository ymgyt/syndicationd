use std::time::Duration;

use anyhow::Context as _;
use synd_runtime::{
    ApiClientConfig, DaemonLaunchCommand, DaemonLaunchConfig, DaemonLaunchLog, DaemonStatus,
    Runtime, RuntimeConfig, RuntimeDatabase, Session, SessionConfig, SessionRequirements,
    ShutdownResult,
};

use crate::config::ConfigResolver;

const SESSION_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const API_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub(crate) struct FeedRuntime {
    runtime: Runtime,
}

impl FeedRuntime {
    pub(crate) fn new(config: &ConfigResolver) -> Self {
        Self {
            runtime: Runtime::new(RuntimeConfig::new(
                RuntimeDatabase::sqlite(config.sqlite_db()),
                ApiClientConfig::new(config.api_timeout(), API_USER_AGENT),
                SessionConfig::new(SESSION_ACQUIRE_TIMEOUT),
                DaemonLaunchConfig::new(
                    DaemonLaunchCommand::current_executable()
                        .with_literal("daemon")
                        .with_literal("serve")
                        .with_literal("--sqlite-db")
                        .with_runtime_database_path(),
                    DaemonLaunchLog::file(config.log_file()),
                ),
                SessionRequirements::default(),
            )),
        }
    }

    pub(crate) async fn acquire_session(&self) -> anyhow::Result<Session> {
        self.runtime
            .acquire_session()
            .await
            .context("Failed to acquire runtime session")
    }

    pub(crate) async fn inspect_daemon(&self) -> anyhow::Result<DaemonStatus> {
        self.runtime
            .daemon()
            .inspect()
            .await
            .context("Failed to inspect runtime daemon")
    }

    pub(crate) async fn shutdown_daemon(&self) -> anyhow::Result<ShutdownResult> {
        self.runtime
            .daemon()
            .shutdown()
            .await
            .context("Failed to shutdown runtime daemon")
    }
}
