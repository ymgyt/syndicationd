#[cfg(unix)]
use std::{
    io::ErrorKind,
    os::unix::{fs::FileTypeExt, net::UnixStream},
    path::{Path, PathBuf},
};

use synd_api::{
    cli::ServeOptions,
    serve::{self, auth::Authenticator},
    shutdown::Shutdown,
};

#[cfg(unix)]
use tokio::net::UnixListener;

use crate::{
    Result, RuntimeDatabase,
    api::RuntimeApiService,
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
            Err(crate::Error::NotImplemented("Daemon::serve on non-Unix"))
        }
    }

    #[cfg(unix)]
    async fn serve_unix(self, placement: RuntimePlacement) -> Result<()> {
        let bound_endpoint = DaemonEndpointBinder::new(placement.endpoint()).bind()?;
        let (listener, endpoint_cleanup) = bound_endpoint.into_parts();
        let shutdown_endpoint_cleanup = endpoint_cleanup.clone();
        let shutdown = Shutdown::manual(move || {
            if let Err(error) = shutdown_endpoint_cleanup.unlink_socket() {
                tracing::warn!(
                    error = %error,
                    "Failed to cleanup daemon endpoint during shutdown"
                );
            }
            tracing::info!("Gracefully shutdown synd-runtime daemon");
        });
        let api_service = RuntimeApiService::from_database(
            self.config.database(),
            Authenticator::trusted_local(),
            ServeOptions::default(),
            &shutdown,
        )
        .await?;
        let (dependency, _registry_runtime) = api_service.into_parts();

        // Keep the registry runtime alive for the entire serve future; dropping it aborts
        // event workers, refresh executor, and scheduler tasks.
        serve::serve_unix(listener, dependency, shutdown).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    database: RuntimeDatabase,
    placement_environment: RuntimePlacementEnvironment,
}

impl DaemonConfig {
    pub fn new(database: RuntimeDatabase) -> Self {
        Self {
            database,
            placement_environment: RuntimePlacementEnvironment::capture(),
        }
    }

    pub fn database(&self) -> &RuntimeDatabase {
        &self.database
    }

    pub(crate) fn placement_environment(&self) -> RuntimePlacementEnvironment {
        self.placement_environment.clone()
    }

    #[cfg(test)]
    fn with_placement_environment(
        mut self,
        placement_environment: RuntimePlacementEnvironment,
    ) -> Self {
        self.placement_environment = placement_environment;
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
                tracing::debug!(
                    daemon_endpoint = %self.path.display(),
                    "Removed daemon endpoint"
                );
            }
            DaemonEndpointFileState::NonSocket => {
                return Err(anyhow::anyhow!(
                    "refusing to remove non-socket daemon endpoint {}",
                    self.path.display()
                )
                .into());
            }
        }

        Ok(())
    }

    fn cleanup_stale_socket(&self) -> Result<()> {
        match DaemonEndpointFileState::inspect(&self.path)? {
            DaemonEndpointFileState::StaleSocket => {
                std::fs::remove_file(&self.path)?;
                tracing::debug!(
                    daemon_endpoint = %self.path.display(),
                    "Removed stale daemon endpoint"
                );
            }
            DaemonEndpointFileState::Missing | DaemonEndpointFileState::ConnectedSocket => {}
            DaemonEndpointFileState::NonSocket => {
                return Err(anyhow::anyhow!(
                    "refusing to remove non-socket daemon endpoint {}",
                    self.path.display()
                )
                .into());
            }
        }

        Ok(())
    }
}

#[cfg(unix)]
impl Drop for DaemonEndpointCleanup {
    fn drop(&mut self) {
        if let Err(error) = self.cleanup_stale_socket() {
            tracing::warn!(
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

    use crate::{
        ApiClientConfig, DaemonLaunchCommand, DaemonLaunchConfig, DaemonLaunchLog, DaemonState,
        Runtime, RuntimeConfig, RuntimeDatabase, SessionConfig, SessionRequirements,
        instance::RuntimeInstance,
        placement::{RuntimePlacementEnvironment, RuntimePlacementResolver, RuntimeRoot},
        uds::UdsEndpoint,
    };

    use super::{Daemon, DaemonConfig, DaemonEndpointBinder};

    #[cfg(unix)]
    struct StartedDaemon {
        probe: DaemonLifecycleProbe,
        daemon_task: tokio::task::JoinHandle<crate::Result<()>>,
    }

    #[cfg(unix)]
    impl StartedDaemon {
        fn spawn(root: &Path) -> Self {
            let database = RuntimeDatabase::sqlite(root.join("synd.db"));
            let placement_environment =
                RuntimePlacementEnvironment::new(RuntimeRoot::from(root.join("runtime")));
            let placement =
                RuntimePlacementResolver::with_environment(placement_environment.clone())
                    .resolve_database(&database)
                    .unwrap();
            let runtime = Runtime::new(
                RuntimeConfig::new(
                    database.clone(),
                    ApiClientConfig::new(Duration::from_secs(2), "synd-runtime-test"),
                    SessionConfig::new(Duration::from_secs(2)),
                    DaemonLaunchConfig::new(
                        DaemonLaunchCommand::executable("unused"),
                        DaemonLaunchLog::file(root.join("daemon.log")),
                    ),
                    SessionRequirements::default(),
                )
                .with_placement_environment(placement_environment.clone()),
            );
            let daemon = Daemon::new(
                DaemonConfig::new(database).with_placement_environment(placement_environment),
            );
            let probe =
                DaemonLifecycleProbe::new(runtime, placement.endpoint().path().to_path_buf());
            let daemon_task = tokio::spawn(daemon.serve());

            Self { probe, daemon_task }
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
    #[tokio::test]
    async fn daemon_shutdown_stops_serving_endpoint() {
        let tmp = tempfile::tempdir().unwrap();
        let daemon = StartedDaemon::spawn(tmp.path());

        daemon.wait_until_running().await;
        daemon.shutdown().await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn daemon_accepts_and_closes_runtime_session() {
        let tmp = tempfile::tempdir().unwrap();
        let daemon = StartedDaemon::spawn(tmp.path());

        daemon.wait_until_running().await;
        let session = daemon.probe.runtime.acquire_session().await.unwrap();
        assert!(session.capabilities().is_empty());
        session.close().await.unwrap();

        daemon.shutdown().await;
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn daemon_endpoint_binder_creates_parent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let instance =
            RuntimeInstance::from_database(&RuntimeDatabase::sqlite(tmp.path().join("synd.db")))
                .unwrap();
        let endpoint = UdsEndpoint::from_instance_id(&tmp.path().join("runtime"), instance.id());

        let _bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();

        assert!(endpoint.path().exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn daemon_endpoint_cleanup_removes_stale_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let instance =
            RuntimeInstance::from_database(&RuntimeDatabase::sqlite(tmp.path().join("synd.db")))
                .unwrap();
        let endpoint = UdsEndpoint::from_instance_id(tmp.path(), instance.id());
        let bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();
        let (listener, cleanup) = bound_endpoint.into_parts();

        drop(listener);
        cleanup.cleanup_stale_socket().unwrap();

        assert!(!endpoint.path().exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn daemon_endpoint_cleanup_keeps_connected_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let instance =
            RuntimeInstance::from_database(&RuntimeDatabase::sqlite(tmp.path().join("synd.db")))
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

    #[cfg(unix)]
    #[tokio::test]
    async fn daemon_endpoint_cleanup_refuses_non_socket_file() {
        let tmp = tempfile::tempdir().unwrap();
        let instance =
            RuntimeInstance::from_database(&RuntimeDatabase::sqlite(tmp.path().join("synd.db")))
                .unwrap();
        let endpoint = UdsEndpoint::from_instance_id(tmp.path(), instance.id());
        let bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();
        let (listener, cleanup) = bound_endpoint.into_parts();

        drop(listener);
        std::fs::remove_file(endpoint.path()).unwrap();
        std::fs::write(endpoint.path(), "").unwrap();
        let error = cleanup.cleanup_stale_socket().unwrap_err();

        assert!(error.to_string().contains("non-socket daemon endpoint"));
        assert!(endpoint.path().exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn daemon_endpoint_shutdown_cleanup_removes_connected_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let instance =
            RuntimeInstance::from_database(&RuntimeDatabase::sqlite(tmp.path().join("synd.db")))
                .unwrap();
        let endpoint = UdsEndpoint::from_instance_id(tmp.path(), instance.id());
        let bound_endpoint = DaemonEndpointBinder::new(&endpoint).bind().unwrap();
        let (_listener, cleanup) = bound_endpoint.into_parts();

        cleanup.unlink_socket().unwrap();

        assert!(!endpoint.path().exists());
    }
}
