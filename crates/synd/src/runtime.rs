use std::time::Duration;

use anyhow::Context as _;
use synd_client::Client;
use synd_runtime::{
    DaemonStatus, Runtime, RuntimeConfig, RuntimeDatabase, Session, ShutdownResult,
};
use tracing::warn;

use crate::config::ConfigResolver;

const SESSION_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);
const API_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"));

pub(crate) struct FeedRuntime {
    runtime: Runtime,
}

impl FeedRuntime {
    pub(crate) fn new(config: &ConfigResolver) -> anyhow::Result<Self> {
        let mut runtime_config = RuntimeConfig::new(RuntimeDatabase::sqlite(config.sqlite_db()))
            .with_api_timeout(config.api_timeout(), API_USER_AGENT)
            .with_session_timeout(SESSION_ACQUIRE_TIMEOUT)
            .with_daemon_log(config.log_file())
            .with_daemon_session_lease_duration(config.daemon_session_lease_duration())
            .with_daemon_session_idle_shutdown_grace(config.daemon_session_idle_shutdown_grace());
        if let Some(root) = config.daemon_runtime_root() {
            runtime_config = runtime_config.with_runtime_root(root);
        }
        let runtime =
            Runtime::try_new(runtime_config).context("Failed to resolve runtime placement")?;

        Ok(Self { runtime })
    }

    pub(crate) async fn acquire_session(&self) -> anyhow::Result<Session> {
        self.runtime
            .acquire_session()
            .await
            .context("Failed to acquire runtime session")
    }

    /// Keeps the daemon session open for a CLI request and closes it even if the request fails.
    pub(crate) async fn with_session<T, E>(
        &self,
        request: impl AsyncFnOnce(&Client) -> Result<T, E>,
    ) -> anyhow::Result<T>
    where
        E: Into<anyhow::Error>,
    {
        let session = self.acquire_session().await?;
        let result = request(session.client()).await;
        if let Err(err) = session.close().await {
            warn!("Failed to close runtime session: {err}");
        }
        result.map_err(Into::into)
    }

    pub(crate) fn into_runtime(self) -> Runtime {
        self.runtime
    }

    pub(crate) async fn inspect_daemon(&self) -> anyhow::Result<DaemonStatus> {
        self.runtime
            .daemon()
            .inspect()
            .await
            .context("Failed to inspect runtime daemon")
    }

    pub(crate) async fn shutdown_daemon(&self, force: bool) -> anyhow::Result<ShutdownResult> {
        let result = if force {
            self.runtime.daemon().force_shutdown().await
        } else {
            self.runtime.daemon().shutdown().await
        };

        result.context("Failed to shutdown runtime daemon")
    }
}
