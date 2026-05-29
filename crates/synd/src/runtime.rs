use std::time::Duration;

use anyhow::Context as _;
use synd_runtime::{
    ApiClientConfig, DaemonLaunchConfig, Runtime, RuntimeConfig, RuntimeDatabase, Session,
    SessionConfig, SessionRequirements,
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
                DaemonLaunchConfig::default(),
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
}
