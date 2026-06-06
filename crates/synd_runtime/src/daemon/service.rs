#[cfg(unix)]
use std::{
    io::ErrorKind,
    os::unix::{fs::FileTypeExt, net::UnixStream},
    path::Path,
};
use std::{path::PathBuf, time::Duration};

#[cfg(test)]
use synd_api::session::DaemonSessionLeasePolicy;
use synd_api::{
    serve::{self, auth::Authenticator},
    session::DaemonSessionConfig,
    shutdown::Shutdown,
};

#[cfg(unix)]
use tokio::net::UnixListener;
use tracing::{debug, info, warn};

use crate::{
    Error, Result, RuntimeDatabase,
    api::ApiService,
    placement::{RuntimePlacement, RuntimePlacementEnvironment, RuntimePlacementResolver},
};

#[cfg(unix)]
use crate::uds::UdsEndpoint;

#[derive(Debug, Clone)]
pub struct Daemon {
    config: DaemonConfig,
}

impl Daemon {
    pub fn new(config: DaemonConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &DaemonConfig {
        &self.config
    }

    pub async fn serve(self) -> Result<()> {
        let placement =
            RuntimePlacementResolver::with_environment(self.config.placement_environment())
                .resolve_database(self.config.database())?;

        self.serve_placement(placement).await
    }

    async fn serve_placement(self, placement: RuntimePlacement) -> Result<()> {
        #[cfg(unix)]
        {
            self.serve_unix(placement).await
        }

        #[cfg(not(unix))]
        {
            Err(crate::Error::UnsupportedTransport {
                context: "daemon service endpoint",
            })
        }
    }

    #[cfg(unix)]
    async fn serve_unix(self, placement: RuntimePlacement) -> Result<()> {
        let bound_endpoint = DaemonEndpointBinder::new(placement.endpoint()).bind()?;
        let (listener, endpoint_cleanup) = bound_endpoint.into_parts();
        let shutdown_endpoint_cleanup = endpoint_cleanup.clone();
        let shutdown = Shutdown::manual(move || {
            if let Err(error) = shutdown_endpoint_cleanup.unlink_socket() {
                warn!(
                    error = %error,
                    "Failed to cleanup daemon endpoint during shutdown"
                );
            }
            info!("Gracefully shutdown synd-runtime daemon");
        });
        let api_service = ApiService::from_database(
            self.config.database(),
            Authenticator::trusted_local(),
            self.config.serve_options(),
            &shutdown,
        )
        .await?;
        let (dependency, _event_workers) = api_service.into_parts();

        // Keep event workers alive for the entire serve future.
        serve::serve_unix(listener, dependency, shutdown).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    database: RuntimeDatabase,
    session: DaemonSessionConfig,
    placement_environment: RuntimePlacementEnvironment,
    #[cfg(test)]
    session_lease_policy: Option<DaemonSessionLeasePolicy>,
}

impl DaemonConfig {
    pub fn new(database: RuntimeDatabase) -> Self {
        Self {
            database,
            session: DaemonSessionConfig::default(),
            placement_environment: RuntimePlacementEnvironment::capture(),
            #[cfg(test)]
            session_lease_policy: None,
        }
    }

    pub fn database(&self) -> &RuntimeDatabase {
        &self.database
    }

    pub(crate) fn placement_environment(&self) -> RuntimePlacementEnvironment {
        self.placement_environment.clone()
    }

    #[must_use]
    pub fn with_session_lease_duration(mut self, lease_duration: Duration) -> Self {
        self.session = self.session.with_lease_duration(lease_duration);
        self
    }

    #[must_use]
    pub fn with_session_idle_shutdown_grace(mut self, idle_shutdown_grace: Duration) -> Self {
        self.session = self.session.with_idle_shutdown_grace(idle_shutdown_grace);
        self
    }

    #[must_use]
    pub fn with_runtime_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.placement_environment = RuntimePlacementEnvironment::from_root(root);
        self
    }

    fn serve_options(&self) -> serve::ServeOptions {
        #[cfg(test)]
        let session = match self.session_lease_policy {
            Some(lease_policy) => {
                DaemonSessionConfig::new(lease_policy, self.session.idle_shutdown_grace())
            }
            None => self.session,
        };

        #[cfg(not(test))]
        let session = self.session;

        serve::ServeOptions::default().with_daemon_sessions(session)
    }

    #[cfg(test)]
    fn with_placement_environment(
        mut self,
        placement_environment: RuntimePlacementEnvironment,
    ) -> Self {
        self.placement_environment = placement_environment;
        self
    }

    #[cfg(test)]
    fn with_session_lease_policy(mut self, lease_policy: DaemonSessionLeasePolicy) -> Self {
        self.session_lease_policy = Some(lease_policy);
        self
    }
}

/// Binds the Unix domain socket endpoint for a daemon.
#[cfg(unix)]
struct DaemonEndpointBinder<'a> {
    endpoint: &'a UdsEndpoint,
}

#[cfg(unix)]
impl<'a> DaemonEndpointBinder<'a> {
    fn new(endpoint: &'a UdsEndpoint) -> Self {
        Self { endpoint }
    }

    fn bind(&self) -> Result<BoundDaemonEndpoint> {
        if let Some(parent) = self.endpoint.path().parent() {
            std::fs::create_dir_all(parent)?;
        }

        Ok(BoundDaemonEndpoint {
            listener: UnixListener::bind(self.endpoint.path())?,
            cleanup: DaemonEndpointCleanup::new(self.endpoint.path().to_path_buf()),
        })
    }
}

#[cfg(unix)]
struct BoundDaemonEndpoint {
    listener: UnixListener,
    cleanup: DaemonEndpointCleanup,
}

#[cfg(unix)]
impl BoundDaemonEndpoint {
    fn into_parts(self) -> (UnixListener, DaemonEndpointCleanup) {
        (self.listener, self.cleanup)
    }
}

/// Best-effort cleanup for a daemon endpoint path owned by this runtime.
#[cfg(unix)]
#[derive(Clone)]
struct DaemonEndpointCleanup {
    path: PathBuf,
}

#[cfg(unix)]
impl DaemonEndpointCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn unlink_socket(&self) -> Result<()> {
        match DaemonEndpointFileState::inspect_file(&self.path)? {
            DaemonEndpointFileState::Missing => {}
            DaemonEndpointFileState::ConnectedSocket | DaemonEndpointFileState::StaleSocket => {
                std::fs::remove_file(&self.path)?;
                debug!(
                    daemon_endpoint = %self.path.display(),
                    "Removed daemon endpoint"
                );
            }
            DaemonEndpointFileState::NonSocket => {
                return Err(Error::NonSocketEndpoint {
                    path: self.path.clone(),
                });
            }
        }

        Ok(())
    }

    fn cleanup_stale_socket(&self) -> Result<()> {
        match DaemonEndpointFileState::inspect(&self.path)? {
            DaemonEndpointFileState::StaleSocket => {
                std::fs::remove_file(&self.path)?;
                debug!(
                    daemon_endpoint = %self.path.display(),
                    "Removed stale daemon endpoint"
                );
            }
            DaemonEndpointFileState::Missing | DaemonEndpointFileState::ConnectedSocket => {}
            DaemonEndpointFileState::NonSocket => {
                return Err(Error::NonSocketEndpoint {
                    path: self.path.clone(),
                });
            }
        }

        Ok(())
    }
}

#[cfg(unix)]
impl Drop for DaemonEndpointCleanup {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup_stale_socket() {
            warn!(
                daemon_endpoint = %self.path.display(),
                error = %error,
                "Failed to cleanup daemon endpoint"
            );
        }
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DaemonEndpointFileState {
    Missing,
    ConnectedSocket,
    StaleSocket,
    NonSocket,
}

#[cfg(unix)]
impl DaemonEndpointFileState {
    fn inspect_file(path: &Path) -> Result<Self> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::Missing),
            Err(error) => return Err(error.into()),
        };

        if !metadata.file_type().is_socket() {
            return Ok(Self::NonSocket);
        }

        Ok(Self::StaleSocket)
    }

    fn inspect(path: &Path) -> Result<Self> {
        match Self::inspect_file(path)? {
            Self::StaleSocket => {}
            state => return Ok(state),
        }

        match UnixStream::connect(path) {
            Ok(_) => Ok(Self::ConnectedSocket),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(Self::Missing),
            Err(error) if error.kind() == ErrorKind::ConnectionRefused => Ok(Self::StaleSocket),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        path::{Path, PathBuf},
        time::Duration,
    };

    use synd_api::session::DaemonSessionLeasePolicy;
    use synd_protocol::session::SessionId;
    use tokio::sync::mpsc;

    use crate::{
        DaemonExecutable, DaemonLaunchConfig, DaemonLaunchLog, DaemonState, Runtime, RuntimeConfig,
        RuntimeDatabase, SessionConfig, SessionRequirements,
        instance::RuntimeInstance,
        placement::{RuntimePlacementEnvironment, RuntimePlacementResolver, RuntimeRoot},
        session::SessionRenewalObserver,
        uds::UdsEndpoint,
    };

    use super::{Daemon, DaemonConfig, DaemonEndpointBinder};

    #[cfg(unix)]
    #[derive(Debug, Default)]
    struct StartedDaemonConfig {
        session_lease_policy: Option<DaemonSessionLeasePolicy>,
        renewal_observer: Option<SessionRenewalObserver>,
        session_requirements: Option<SessionRequirements>,
    }

    #[cfg(unix)]
    impl StartedDaemonConfig {
        fn with_session_lease_policy(mut self, lease_policy: DaemonSessionLeasePolicy) -> Self {
            self.session_lease_policy = Some(lease_policy);
            self
        }

        fn with_renewal_observer(mut self, observer: SessionRenewalObserver) -> Self {
            self.renewal_observer = Some(observer);
            self
        }

        fn with_session_requirements(mut self, requirements: SessionRequirements) -> Self {
            self.session_requirements = Some(requirements);
            self
        }
    }

    #[cfg(unix)]
    struct SessionRenewalProbe {
        renewed: mpsc::UnboundedReceiver<SessionId>,
    }

    #[cfg(unix)]
    impl SessionRenewalProbe {
        fn new() -> (SessionRenewalObserver, Self) {
            let (renewed_tx, renewed_rx) = mpsc::unbounded_channel();

            (
                SessionRenewalObserver::new(renewed_tx),
                Self {
                    renewed: renewed_rx,
                },
            )
        }

        async fn wait_for_renewals(
            &mut self,
            expected_renewals: usize,
            timeout: Duration,
        ) -> Vec<SessionId> {
            tokio::time::timeout(timeout, async {
                let mut renewals = Vec::with_capacity(expected_renewals);
                while renewals.len() < expected_renewals {
                    let session_id = self
                        .renewed
                        .recv()
                        .await
                        .expect("session renewal observer closed before expected renewal");
                    renewals.push(session_id);
                }
                renewals
            })
            .await
            .expect("timed out waiting for session renewals")
        }
    }

    #[cfg(unix)]
    struct StartedDaemon {
        probe: DaemonLifecycleProbe,
        daemon_task: tokio::task::JoinHandle<crate::Result<()>>,
    }

    #[cfg(unix)]
    impl StartedDaemon {
        fn spawn(root: &Path) -> crate::Result<Self> {
            Self::spawn_with_config(root, StartedDaemonConfig::default())
        }

        fn spawn_with_config(root: &Path, config: StartedDaemonConfig) -> crate::Result<Self> {
            let database = RuntimeDatabase::sqlite(root.join("synd.db"));
            let placement_environment =
                RuntimePlacementEnvironment::new(RuntimeRoot::from(root.join("runtime")));
            let placement =
                RuntimePlacementResolver::with_environment(placement_environment.clone())
                    .resolve_database(&database)?;
            let session_config = {
                let session_config = SessionConfig::new(Duration::from_secs(2));
                match config.renewal_observer {
                    Some(observer) => session_config.with_renewal_observer(observer),
                    None => session_config,
                }
            };
            let runtime_config = {
                let runtime_config = RuntimeConfig::new(database.clone())
                    .with_api_timeout(Duration::from_secs(2), "synd-runtime-test")
                    .with_session(session_config)
                    .with_daemon_launch(DaemonLaunchConfig::new(
                        DaemonExecutable::path("unused"),
                        DaemonLaunchLog::file(root.join("daemon.log")),
                    ))
                    .with_placement_environment(placement_environment.clone());
                match config.session_requirements {
                    Some(requirements) => runtime_config.with_requirements(requirements),
                    None => runtime_config,
                }
            };
            let runtime = Runtime::try_new(runtime_config)?;
            let mut daemon_config =
                DaemonConfig::new(database).with_placement_environment(placement_environment);
            if let Some(lease_policy) = config.session_lease_policy {
                daemon_config = daemon_config.with_session_lease_policy(lease_policy);
            }
            let daemon = Daemon::new(daemon_config);
            let probe =
                DaemonLifecycleProbe::new(runtime, placement.endpoint().path().to_path_buf());
            let daemon_task = tokio::spawn(daemon.serve());

            Ok(Self { probe, daemon_task })
        }

        async fn wait_until_running(&self) {
            self.probe.wait_until_running().await;
        }

        async fn shutdown(self) {
            self.probe.shutdown().await;
            tokio::time::timeout(Duration::from_secs(5), self.daemon_task)
                .await
                .unwrap()
                .unwrap()
                .unwrap();

            assert!(!self.probe.endpoint.exists());
        }
    }

    #[cfg(unix)]
    struct DaemonLifecycleProbe {
        runtime: Runtime,
        endpoint: PathBuf,
    }

    #[cfg(unix)]
    impl DaemonLifecycleProbe {
        fn new(runtime: Runtime, endpoint: PathBuf) -> Self {
            Self { runtime, endpoint }
        }

        async fn wait_until_running(&self) {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    let status = self.runtime.daemon().inspect().await.unwrap();
                    if status.state() == DaemonState::Running {
                        return;
                    }

                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            })
            .await
            .unwrap();
        }

        async fn shutdown(&self) {
            let result = self.runtime.daemon().shutdown().await.unwrap();

            assert_eq!(result.status().state(), DaemonState::NotRunning);
            assert_eq!(
                result.status().placement().endpoint(),
                self.endpoint.as_path()
            );
        }
    }

    #[cfg(unix)]
    mod shutdown {
        use super::*;

        #[tokio::test]
        async fn stops_endpoint() -> crate::Result<()> {
            let tmp = tempfile::tempdir()?;
            let daemon = StartedDaemon::spawn(tmp.path())?;

            daemon.wait_until_running().await;
            daemon.shutdown().await;
            Ok(())
        }
    }

    #[cfg(unix)]
    mod session {
        use super::*;

        mod lifecycle {
            use super::*;

            #[tokio::test]
            async fn accepts_and_closes() -> crate::Result<()> {
                let tmp = tempfile::tempdir()?;
                let daemon = StartedDaemon::spawn(tmp.path())?;

                daemon.wait_until_running().await;
                let session = daemon.probe.runtime.acquire_session().await?;
                assert_eq!(
                    session.available_capabilities(),
                    &synd_protocol::capability::local_api_capabilities()
                );
                session.close().await?;

                daemon.shutdown().await;
                Ok(())
            }
        }

        mod required_capabilities {
            use super::*;

            #[tokio::test]
            async fn rejects_missing() -> crate::Result<()> {
                let tmp = tempfile::tempdir()?;
                let missing_capabilities = synd_protocol::CapabilitySet::new(["test.missing"]);
                let daemon = StartedDaemon::spawn_with_config(
                    tmp.path(),
                    StartedDaemonConfig::default().with_session_requirements(
                        SessionRequirements::new(missing_capabilities.clone()),
                    ),
                )?;

                daemon.wait_until_running().await;
                let unexpected = match daemon.probe.runtime.acquire_session().await {
                    Err(crate::Error::MissingSessionCapabilities {
                        missing_capabilities: actual,
                        ..
                    }) => {
                        assert_eq!(actual, missing_capabilities);
                        None
                    }
                    Err(error) => Some(format!("unexpected acquire_session error: {error:?}")),
                    Ok(session) => {
                        session.close().await?;
                        Some("session unexpectedly opened".to_owned())
                    }
                };

                daemon.shutdown().await;
                if let Some(message) = unexpected {
                    panic!("{message}");
                }

                Ok(())
            }
        }

        mod lease {
            use super::*;

            #[tokio::test]
            async fn renews() -> crate::Result<()> {
                let tmp = tempfile::tempdir()?;
                let lease_policy = DaemonSessionLeasePolicy::new(
                    Duration::from_secs(2),
                    Duration::from_millis(200),
                );
                let (observer, mut renewal_probe) = SessionRenewalProbe::new();
                let daemon = StartedDaemon::spawn_with_config(
                    tmp.path(),
                    StartedDaemonConfig::default()
                        .with_session_lease_policy(lease_policy)
                        .with_renewal_observer(observer),
                )?;

                daemon.wait_until_running().await;
                let session = daemon.probe.runtime.acquire_session().await?;
                let renewed_session_ids = renewal_probe
                    .wait_for_renewals(4, Duration::from_secs(8))
                    .await;
                let first_session_id = &renewed_session_ids[0];

                assert!(
                    renewed_session_ids
                        .iter()
                        .all(|session_id| session_id == first_session_id)
                );
                session.close().await?;

                daemon.shutdown().await;
                Ok(())
            }
        }
    }

    #[cfg(unix)]
    mod endpoint_binding {
        use super::*;

        #[tokio::test]
        async fn creates_parent_dir() {
            let tmp = tempfile::tempdir().unwrap();
            let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(
                tmp.path().join("synd.db"),
            ))
            .unwrap();
            let endpoint =
                UdsEndpoint::from_instance_id(&tmp.path().join("runtime"), instance.id());

            let _bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();

            assert!(endpoint.path().exists());
        }
    }

    #[cfg(unix)]
    mod endpoint_cleanup {
        use super::*;

        mod stale_socket {
            use super::*;

            #[tokio::test]
            async fn removes_socket() {
                let tmp = tempfile::tempdir().unwrap();
                let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(
                    tmp.path().join("synd.db"),
                ))
                .unwrap();
                let endpoint = UdsEndpoint::from_instance_id(tmp.path(), instance.id());
                let bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();
                let (listener, cleanup) = bound_endpoint.into_parts();

                drop(listener);
                cleanup.cleanup_stale_socket().unwrap();

                assert!(!endpoint.path().exists());
            }

            #[tokio::test]
            async fn keeps_connected_socket() {
                let tmp = tempfile::tempdir().unwrap();
                let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(
                    tmp.path().join("synd.db"),
                ))
                .unwrap();
                let endpoint = UdsEndpoint::from_instance_id(tmp.path(), instance.id());
                let bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();
                let (listener, cleanup) = bound_endpoint.into_parts();

                drop(listener);
                std::fs::remove_file(endpoint.path()).unwrap();
                let _replacement = std::os::unix::net::UnixListener::bind(endpoint.path()).unwrap();
                cleanup.cleanup_stale_socket().unwrap();

                assert!(endpoint.path().exists());
            }

            #[tokio::test]
            async fn refuses_non_socket_file() {
                let tmp = tempfile::tempdir().unwrap();
                let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(
                    tmp.path().join("synd.db"),
                ))
                .unwrap();
                let endpoint = UdsEndpoint::from_instance_id(tmp.path(), instance.id());
                let bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();
                let (listener, cleanup) = bound_endpoint.into_parts();

                drop(listener);
                std::fs::remove_file(endpoint.path()).unwrap();
                std::fs::write(endpoint.path(), "").unwrap();
                let error = cleanup.cleanup_stale_socket().unwrap_err();

                assert!(error.to_string().contains("non-socket runtime endpoint"));
                assert!(endpoint.path().exists());
            }
        }

        mod shutdown {
            use super::*;

            #[tokio::test]
            async fn removes_socket() {
                let tmp = tempfile::tempdir().unwrap();
                let instance = RuntimeInstance::from_database(&RuntimeDatabase::sqlite(
                    tmp.path().join("synd.db"),
                ))
                .unwrap();
                let endpoint = UdsEndpoint::from_instance_id(tmp.path(), instance.id());
                let bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();
                let (_listener, cleanup) = bound_endpoint.into_parts();

                cleanup.unlink_socket().unwrap();

                assert!(!endpoint.path().exists());
            }
        }
    }
}
