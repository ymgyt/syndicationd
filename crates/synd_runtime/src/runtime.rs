use std::time::Duration;

use crate::{
    DaemonControl, DaemonLaunchConfig, Result, RuntimeDatabase, Session, SessionConfig,
    SessionRequirements, acquisition::SessionAcquisition, placement::RuntimePlacementEnvironment,
};

#[derive(Debug, Clone)]
pub struct Runtime {
    config: Config,
}

impl Runtime {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub async fn acquire_session(&self) -> Result<Session> {
        SessionAcquisition::new(&self.config).acquire().await
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
    placement_environment: RuntimePlacementEnvironment,
}

impl Config {
    pub fn new(
        database: RuntimeDatabase,
        client: ApiClientConfig,
        session: SessionConfig,
        daemon: DaemonLaunchConfig,
        requirements: SessionRequirements,
    ) -> Self {
        Self {
            database,
            client,
            session,
            daemon,
            requirements,
            placement_environment: RuntimePlacementEnvironment::capture(),
        }
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

    pub(crate) fn placement_environment(&self) -> RuntimePlacementEnvironment {
        self.placement_environment.clone()
    }

    #[cfg(test)]
    pub(crate) fn with_placement_environment(
        mut self,
        placement_environment: RuntimePlacementEnvironment,
    ) -> Self {
        self.placement_environment = placement_environment;
        self
    }
}

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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::{
        ApiClientConfig, DaemonLaunchConfig, Runtime, RuntimeConfig, RuntimeDatabase,
        SessionConfig, SessionRequirements,
    };

    #[test]
    fn runtime_keeps_daemon_launch_config() {
        let tempdir = tempfile::tempdir().unwrap();
        let config = RuntimeConfig::new(
            RuntimeDatabase::sqlite(tempdir.path().join("synd.db")),
            ApiClientConfig::new(Duration::from_secs(5), "synd-runtime-test"),
            SessionConfig::new(Duration::from_secs(5)),
            DaemonLaunchConfig::default(),
            SessionRequirements::default(),
        );

        let runtime = Runtime::new(config.clone());

        assert_eq!(runtime.config().daemon(), config.daemon());
    }
}
