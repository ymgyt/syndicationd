use std::{path::PathBuf, time::Duration};

use crate::{
    DaemonControl, DaemonLaunchConfig, DaemonLaunchLog, PlacementSummary, Result, RuntimeDatabase,
    Session, SessionConfig, SessionRequirements,
    acquisition::SessionAcquisition,
    placement::{PlacementEnvironment, PlacementResolver, PlacementSpec},
};

#[derive(Debug, Clone)]
pub struct Runtime {
    config: Config,
    placement: PlacementSpec,
}

impl Runtime {
    pub fn try_new(config: Config) -> Result<Self> {
        let placement =
            PlacementResolver::with_environment(config.placement_environment()).resolve(&config)?;

        Ok(Self { config, placement })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub(crate) fn placement(&self) -> &PlacementSpec {
        &self.placement
    }

    pub fn placement_summary(&self) -> PlacementSummary {
        PlacementSummary::from_placement(&self.placement)
    }

    pub async fn acquire_session(&self) -> Result<Session> {
        SessionAcquisition::new(self).acquire().await
    }

    pub fn daemon(&self) -> DaemonControl<'_> {
        DaemonControl::new(self)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    database: RuntimeDatabase,
    client: ApiClientConfig,
    session: SessionConfig,
    daemon: DaemonLaunchConfig,
    requirements: SessionRequirements,
    placement_environment: PlacementEnvironment,
}

impl Config {
    pub fn new(database: RuntimeDatabase) -> Self {
        Self {
            database,
            client: ApiClientConfig::default(),
            session: SessionConfig::default(),
            daemon: DaemonLaunchConfig::default(),
            requirements: SessionRequirements::default(),
            placement_environment: PlacementEnvironment::capture(),
        }
    }

    #[must_use]
    pub fn with_api_client(mut self, client: ApiClientConfig) -> Self {
        self.client = client;
        self
    }

    #[must_use]
    pub fn with_api_timeout(
        self,
        request_timeout: Duration,
        user_agent: impl Into<String>,
    ) -> Self {
        self.with_api_client(ApiClientConfig::new(request_timeout, user_agent))
    }

    #[must_use]
    pub fn with_session(mut self, session: SessionConfig) -> Self {
        self.session = session;
        self
    }

    #[must_use]
    pub fn with_session_timeout(self, acquire_timeout: Duration) -> Self {
        self.with_session(SessionConfig::new(acquire_timeout))
    }

    #[must_use]
    pub fn with_daemon_launch(mut self, daemon: DaemonLaunchConfig) -> Self {
        self.daemon = daemon;
        self
    }

    #[must_use]
    pub fn with_daemon_log(mut self, log: impl Into<PathBuf>) -> Self {
        self.daemon = self.daemon.with_log(DaemonLaunchLog::file(log));
        self
    }

    #[must_use]
    pub fn with_daemon_session_lease_duration(mut self, lease_duration: Duration) -> Self {
        self.daemon = self.daemon.with_session_lease_duration(lease_duration);
        self
    }

    #[must_use]
    pub fn with_daemon_session_idle_shutdown_grace(
        mut self,
        idle_shutdown_grace: Duration,
    ) -> Self {
        self.daemon = self
            .daemon
            .with_session_idle_shutdown_grace(idle_shutdown_grace);
        self
    }

    #[must_use]
    pub fn with_runtime_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.placement_environment = PlacementEnvironment::from_root(root);
        self
    }

    #[must_use]
    pub fn with_requirements(mut self, requirements: SessionRequirements) -> Self {
        self.requirements = requirements;
        self
    }

    pub fn database(&self) -> &RuntimeDatabase {
        &self.database
    }

    pub fn client(&self) -> &ApiClientConfig {
        &self.client
    }

    pub fn session(&self) -> &SessionConfig {
        &self.session
    }

    pub fn daemon(&self) -> &DaemonLaunchConfig {
        &self.daemon
    }

    pub fn requirements(&self) -> &SessionRequirements {
        &self.requirements
    }

    pub(crate) fn placement_environment(&self) -> PlacementEnvironment {
        self.placement_environment.clone()
    }

    #[cfg(test)]
    pub(crate) fn with_placement_environment(
        mut self,
        placement_environment: PlacementEnvironment,
    ) -> Self {
        self.placement_environment = placement_environment;
        self
    }
}

/// Configuration for `synd_api` client
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiClientConfig {
    request_timeout: Duration,
    user_agent: String,
}

impl ApiClientConfig {
    pub fn new(request_timeout: Duration, user_agent: impl Into<String>) -> Self {
        Self {
            request_timeout,
            user_agent: user_agent.into(),
        }
    }

    pub fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    pub fn user_agent(&self) -> &str {
        &self.user_agent
    }
}

impl Default for ApiClientConfig {
    fn default() -> Self {
        Self::new(
            Duration::from_secs(30),
            concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION")),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        DaemonExecutable, DaemonLaunchConfig, DaemonLaunchLog, Runtime, RuntimeConfig,
        RuntimeDatabase,
    };

    mod daemon_launch {
        use super::*;

        #[test]
        fn keeps_config() -> crate::Result<()> {
            let tempdir = tempfile::tempdir()?;
            let config =
                RuntimeConfig::new(RuntimeDatabase::sqlite(tempdir.path().join("synd.db")))
                    .with_api_timeout(Duration::from_secs(5), "synd-runtime-test")
                    .with_session_timeout(Duration::from_secs(5))
                    .with_daemon_launch(DaemonLaunchConfig::default());

            let runtime = Runtime::try_new(config.clone())?;

            assert_eq!(runtime.config().daemon(), config.daemon());
            Ok(())
        }
    }

    mod daemon_log {
        use super::*;

        #[test]
        fn keeps_executable() -> crate::Result<()> {
            let tempdir = tempfile::tempdir()?;
            let log = tempdir.path().join("daemon.log");
            let config =
                RuntimeConfig::new(RuntimeDatabase::sqlite(tempdir.path().join("synd.db")))
                    .with_daemon_launch(DaemonLaunchConfig::new(
                        DaemonExecutable::path("/usr/bin/custom-synd"),
                        DaemonLaunchLog::file(tempdir.path().join("old.log")),
                    ))
                    .with_daemon_log(log.clone());

            assert_eq!(
                config.daemon(),
                &DaemonLaunchConfig::new(
                    DaemonExecutable::path("/usr/bin/custom-synd"),
                    DaemonLaunchLog::file(log),
                )
            );
            Ok(())
        }
    }
}
