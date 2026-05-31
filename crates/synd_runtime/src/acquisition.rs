use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;

use synd_protocol::session::OpenSessionRequest;
use tokio::time::sleep;
use tracing::{debug, info};

use crate::{
    Error, Result, RuntimeConfig, Session,
    connection::{RuntimeEndpointConnectionStatus, RuntimeEndpointConnector},
    daemon::{DaemonHandle, DaemonLauncher},
    placement::{RuntimePlacement, RuntimePlacementResolver},
    startup::{StartupLock, StartupLockAcquirer, StartupLockAcquisition},
};

const ENDPOINT_WAIT_INTERVAL: Duration = Duration::from_millis(50);

/// Controls the ordered decisions required to acquire a runtime session.
pub(crate) struct SessionAcquisition<'a> {
    config: &'a RuntimeConfig,
    placement_resolver: RuntimePlacementResolver,
}

impl<'a> SessionAcquisition<'a> {
    pub(crate) fn new(config: &'a RuntimeConfig) -> Self {
        Self {
            config,
            placement_resolver: RuntimePlacementResolver::with_environment(
                config.placement_environment(),
            ),
        }
    }

    pub(crate) async fn acquire(self) -> Result<Session> {
        let context = self.resolve_context().await?;
        context.trace();

        let decision = SessionAcquisitionDecision::from(context);
        decision.trace();

        self.execute(decision).await
    }

    async fn resolve_context(&self) -> Result<SessionAcquisitionContext> {
        let placement = self.placement_resolver.resolve(self.config)?;
        let endpoint_connection = RuntimeEndpointConnector::new(placement.endpoint())
            .try_connect()
            .await;

        Ok(SessionAcquisitionContext {
            placement,
            endpoint_connection,
        })
    }

    async fn execute(&self, decision: SessionAcquisitionDecision) -> Result<Session> {
        debug!(
            runtime_session_path = decision.path_name(),
            "Executing runtime session acquisition path"
        );

        match decision {
            SessionAcquisitionDecision::AttachExistingRuntime { placement } => {
                RuntimeSessionConnector::new(self.config, placement)
                    .connect()
                    .await
            }
            SessionAcquisitionDecision::StartMissingRuntime { placement }
            | SessionAcquisitionDecision::RecoverStaleRuntime { placement } => {
                RuntimeStartup::new(self.config, placement)
                    .acquire_session()
                    .await
            }
            SessionAcquisitionDecision::FailUnavailableEndpoint { .. } => {
                Err(Error::NotImplemented("runtime endpoint unavailable"))
            }
            #[cfg(not(unix))]
            SessionAcquisitionDecision::FailUnsupportedTransport { .. } => {
                Err(Error::NotImplemented("runtime transport unsupported"))
            }
        }
    }
}

/// Facts collected before selecting the session acquisition path.
#[derive(Debug, Clone)]
struct SessionAcquisitionContext {
    placement: RuntimePlacement,
    endpoint_connection: RuntimeEndpointConnectionStatus,
}

impl SessionAcquisitionContext {
    fn trace(&self) {
        debug!(
            runtime_root = %self.placement.root().path().display(),
            runtime_instance_id = %self.placement.instance().id(),
            runtime_database = %self.placement.instance().canonical_database_path().display(),
            runtime_endpoint = %self.placement.endpoint().path().display(),
            runtime_startup_lock = %self.placement.startup_lock_path().path().display(),
            runtime_endpoint_connection = ?self.endpoint_connection,
            "Resolved runtime session acquisition context"
        );
    }
}

/// Branch selected from the session acquisition context.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionAcquisitionDecision {
    AttachExistingRuntime {
        placement: RuntimePlacement,
    },
    StartMissingRuntime {
        placement: RuntimePlacement,
    },
    RecoverStaleRuntime {
        placement: RuntimePlacement,
    },
    FailUnavailableEndpoint {
        placement: RuntimePlacement,
    },
    #[cfg(not(unix))]
    FailUnsupportedTransport {
        placement: RuntimePlacement,
    },
}

impl From<SessionAcquisitionContext> for SessionAcquisitionDecision {
    fn from(context: SessionAcquisitionContext) -> Self {
        match context.endpoint_connection {
            RuntimeEndpointConnectionStatus::Connected => Self::AttachExistingRuntime {
                placement: context.placement,
            },
            RuntimeEndpointConnectionStatus::Missing => Self::StartMissingRuntime {
                placement: context.placement,
            },
            RuntimeEndpointConnectionStatus::Stale => Self::RecoverStaleRuntime {
                placement: context.placement,
            },
            RuntimeEndpointConnectionStatus::Unavailable => Self::FailUnavailableEndpoint {
                placement: context.placement,
            },
            #[cfg(not(unix))]
            RuntimeEndpointConnectionStatus::UnsupportedTransport => {
                Self::FailUnsupportedTransport {
                    placement: context.placement,
                }
            }
        }
    }
}

impl SessionAcquisitionDecision {
    fn trace(&self) {
        debug!(
            runtime_session_path = self.path_name(),
            "Selected runtime session acquisition path"
        );
    }

    fn path_name(&self) -> &'static str {
        match self {
            Self::AttachExistingRuntime { .. } => "attach_existing_runtime",
            Self::StartMissingRuntime { .. } => "start_missing_runtime",
            Self::RecoverStaleRuntime { .. } => "recover_stale_runtime",
            Self::FailUnavailableEndpoint { .. } => "fail_unavailable_endpoint",
            #[cfg(not(unix))]
            Self::FailUnsupportedTransport { .. } => "fail_unsupported_transport",
        }
    }
}

/// Starts or recovers a runtime while holding the instance startup lock.
struct RuntimeStartup<'a> {
    config: &'a RuntimeConfig,
    placement: RuntimePlacement,
}

impl<'a> RuntimeStartup<'a> {
    fn new(config: &'a RuntimeConfig, placement: RuntimePlacement) -> Self {
        Self { config, placement }
    }

    async fn acquire_session(self) -> Result<Session> {
        match StartupLockAcquirer::new(self.placement.startup_lock_path()).try_acquire()? {
            StartupLockAcquisition::Acquired(startup_lock) => {
                self.acquire_after_lock(startup_lock).await
            }
            StartupLockAcquisition::AlreadyHeld => self.wait_for_existing_startup().await,
            #[cfg(not(unix))]
            StartupLockAcquisition::UnsupportedTransport => {
                Err(Error::NotImplemented("runtime startup lock unsupported"))
            }
        }
    }

    async fn acquire_after_lock(self, startup_lock: StartupLock) -> Result<Session> {
        let context = self.resolve_context().await;
        context.trace();

        let decision = RuntimeStartupDecision::from(context);
        decision.trace();

        self.execute(decision, startup_lock).await
    }

    async fn resolve_context(&self) -> RuntimeStartupContext {
        let endpoint_connection = RuntimeEndpointConnector::new(self.placement.endpoint())
            .try_connect()
            .await;

        RuntimeStartupContext {
            placement: self.placement.clone(),
            endpoint_connection,
        }
    }

    async fn wait_for_existing_startup(self) -> Result<Session> {
        RuntimeEndpointWaiter::new(
            self.placement.clone(),
            self.config.session().acquire_timeout(),
        )
        .wait_until_connected()
        .await?;
        RuntimeSessionConnector::new(self.config, self.placement)
            .connect()
            .await
    }

    async fn execute(
        &self,
        decision: RuntimeStartupDecision,
        _startup_lock: StartupLock,
    ) -> Result<Session> {
        debug!(
            runtime_startup_path = decision.path_name(),
            "Executing runtime startup path"
        );

        // Holding `_startup_lock` in this scope serializes startup/recovery for the
        // runtime instance until the selected path has produced a session or failed.
        match decision {
            RuntimeStartupDecision::AttachStartedRuntime { placement } => {
                RuntimeSessionConnector::new(self.config, placement)
                    .connect()
                    .await
            }
            RuntimeStartupDecision::LaunchMissingRuntime { placement } => {
                RuntimeDaemonStarter::new(self.config, placement)
                    .start()
                    .await
            }
            RuntimeStartupDecision::RecoverStaleEndpoint { placement } => {
                let placement = RuntimeStaleEndpointRecovery::new(placement).recover()?;

                RuntimeDaemonStarter::new(self.config, placement)
                    .start()
                    .await
            }
            RuntimeStartupDecision::FailUnavailableEndpoint { .. } => Err(Error::NotImplemented(
                "runtime endpoint unavailable after startup lock",
            )),
            #[cfg(not(unix))]
            RuntimeStartupDecision::FailUnsupportedTransport { .. } => Err(Error::NotImplemented(
                "runtime transport unsupported after startup lock",
            )),
        }
    }
}

/// Removes a stale Unix socket endpoint while the startup lock is held.
struct RuntimeStaleEndpointRecovery {
    placement: RuntimePlacement,
}

impl RuntimeStaleEndpointRecovery {
    fn new(placement: RuntimePlacement) -> Self {
        Self { placement }
    }

    fn recover(self) -> Result<RuntimePlacement> {
        #[cfg(unix)]
        {
            let endpoint_path = self.placement.endpoint().path();
            match std::fs::symlink_metadata(endpoint_path) {
                Ok(metadata) if metadata.file_type().is_socket() => {
                    std::fs::remove_file(endpoint_path)?;
                    info!(
                        runtime_endpoint = %endpoint_path.display(),
                        "Removed stale runtime endpoint"
                    );
                }
                Ok(_) => {
                    return Err(anyhow::anyhow!(
                        "refusing to remove non-socket runtime endpoint {}",
                        endpoint_path.display()
                    )
                    .into());
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }

            Ok(self.placement)
        }

        #[cfg(not(unix))]
        {
            let _ = self;

            Err(Error::NotImplemented("recovering stale runtime endpoint"))
        }
    }
}

/// Starts the daemon for one resolved runtime placement and returns a session.
struct RuntimeDaemonStarter<'a> {
    config: &'a RuntimeConfig,
    placement: RuntimePlacement,
}

impl<'a> RuntimeDaemonStarter<'a> {
    fn new(config: &'a RuntimeConfig, placement: RuntimePlacement) -> Self {
        Self { config, placement }
    }

    async fn start(self) -> Result<Session> {
        let mut daemon_handle =
            DaemonLauncher::new(self.config.daemon(), self.placement.clone()).launch()?;
        RuntimeEndpointWaiter::new(
            self.placement.clone(),
            self.config.session().acquire_timeout(),
        )
        .wait_for_launched_daemon(&mut daemon_handle)
        .await?;
        daemon_handle.reap_in_background();
        RuntimeSessionConnector::new(self.config, self.placement)
            .connect()
            .await
    }
}

/// Waits until a daemon endpoint is ready to accept connections.
struct RuntimeEndpointWaiter {
    placement: RuntimePlacement,
    timeout: Duration,
}

impl RuntimeEndpointWaiter {
    fn new(placement: RuntimePlacement, timeout: Duration) -> Self {
        Self { placement, timeout }
    }

    async fn wait_until_connected(&self) -> Result<()> {
        self.wait(None).await
    }

    async fn wait_for_launched_daemon(&self, daemon_handle: &mut DaemonHandle) -> Result<()> {
        self.wait(Some(daemon_handle)).await
    }

    async fn wait(&self, mut daemon_handle: Option<&mut DaemonHandle>) -> Result<()> {
        let deadline = Instant::now() + self.timeout;

        loop {
            if let Some(handle) = daemon_handle.as_deref_mut()
                && let Some(status) = handle.try_wait()?
            {
                return Err(anyhow::anyhow!(
                    "daemon exited before endpoint became ready: {status}"
                )
                .into());
            }

            let endpoint_connection = RuntimeEndpointConnector::new(self.placement.endpoint())
                .try_connect()
                .await;
            if endpoint_connection == RuntimeEndpointConnectionStatus::Connected {
                return Ok(());
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(anyhow::anyhow!(
                    "timed out waiting for daemon endpoint {} to become ready",
                    self.placement.endpoint().path().display()
                )
                .into());
            }

            debug!(
                runtime_endpoint = %self.placement.endpoint().path().display(),
                runtime_endpoint_connection = ?endpoint_connection,
                "Waiting for daemon endpoint"
            );
            sleep(ENDPOINT_WAIT_INTERVAL.min(deadline - now)).await;
        }
    }
}

/// Facts collected after acquiring the startup lock.
#[derive(Debug, Clone)]
struct RuntimeStartupContext {
    placement: RuntimePlacement,
    endpoint_connection: RuntimeEndpointConnectionStatus,
}

impl RuntimeStartupContext {
    fn trace(&self) {
        debug!(
            runtime_root = %self.placement.root().path().display(),
            runtime_instance_id = %self.placement.instance().id(),
            runtime_database = %self.placement.instance().canonical_database_path().display(),
            runtime_endpoint = %self.placement.endpoint().path().display(),
            runtime_startup_lock = %self.placement.startup_lock_path().path().display(),
            runtime_endpoint_connection = ?self.endpoint_connection,
            "Resolved runtime startup context after acquiring startup lock"
        );
    }
}

/// Branch selected from the startup-lock-protected runtime context.
#[derive(Debug, Clone, PartialEq, Eq)]
enum RuntimeStartupDecision {
    AttachStartedRuntime {
        placement: RuntimePlacement,
    },
    LaunchMissingRuntime {
        placement: RuntimePlacement,
    },
    RecoverStaleEndpoint {
        placement: RuntimePlacement,
    },
    FailUnavailableEndpoint {
        placement: RuntimePlacement,
    },
    #[cfg(not(unix))]
    FailUnsupportedTransport {
        placement: RuntimePlacement,
    },
}

impl From<RuntimeStartupContext> for RuntimeStartupDecision {
    fn from(context: RuntimeStartupContext) -> Self {
        match context.endpoint_connection {
            RuntimeEndpointConnectionStatus::Connected => Self::AttachStartedRuntime {
                placement: context.placement,
            },
            RuntimeEndpointConnectionStatus::Missing => Self::LaunchMissingRuntime {
                placement: context.placement,
            },
            RuntimeEndpointConnectionStatus::Stale => Self::RecoverStaleEndpoint {
                placement: context.placement,
            },
            RuntimeEndpointConnectionStatus::Unavailable => Self::FailUnavailableEndpoint {
                placement: context.placement,
            },
            #[cfg(not(unix))]
            RuntimeEndpointConnectionStatus::UnsupportedTransport => {
                Self::FailUnsupportedTransport {
                    placement: context.placement,
                }
            }
        }
    }
}

impl RuntimeStartupDecision {
    fn trace(&self) {
        debug!(
            runtime_startup_path = self.path_name(),
            "Selected runtime startup path"
        );
    }

    fn path_name(&self) -> &'static str {
        match self {
            Self::AttachStartedRuntime { .. } => "attach_started_runtime",
            Self::LaunchMissingRuntime { .. } => "launch_missing_runtime",
            Self::RecoverStaleEndpoint { .. } => "recover_stale_endpoint",
            Self::FailUnavailableEndpoint { .. } => "fail_unavailable_endpoint",
            #[cfg(not(unix))]
            Self::FailUnsupportedTransport { .. } => "fail_unsupported_transport",
        }
    }
}

/// Builds a session connected to an already-running runtime endpoint.
struct RuntimeSessionConnector<'a> {
    config: &'a RuntimeConfig,
    placement: RuntimePlacement,
}

impl<'a> RuntimeSessionConnector<'a> {
    fn new(config: &'a RuntimeConfig, placement: RuntimePlacement) -> Self {
        Self { config, placement }
    }

    async fn connect(self) -> Result<Session> {
        #[cfg(unix)]
        {
            let client = synd_client::Client::new_unix(
                self.placement.endpoint().path(),
                synd_client::ClientOptions::new(
                    self.config.client().request_timeout(),
                    self.config.client().user_agent(),
                ),
            )?;
            let session = client
                .open_session(OpenSessionRequest::new(
                    self.config.requirements().capabilities().clone(),
                ))
                .await?;

            Ok(Session::new(
                client.clone(),
                session.capabilities().clone(),
                crate::SessionHandle::daemon(client, session.session_id().clone()),
            ))
        }

        #[cfg(not(unix))]
        {
            Err(Error::NotImplemented(
                "runtime endpoint session on non-Unix",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        RuntimeDatabase,
        connection::RuntimeEndpointConnectionStatus,
        instance::RuntimeInstance,
        placement::{RuntimePlacement, RuntimeRoot},
    };
    #[cfg(unix)]
    use std::os::unix::net::UnixListener;

    use super::{
        RuntimeStaleEndpointRecovery, RuntimeStartupContext, RuntimeStartupDecision,
        SessionAcquisitionContext, SessionAcquisitionDecision,
    };

    #[test]
    fn selects_session_acquisition_decision_from_endpoint_connection() {
        let cases = [
            (
                RuntimeEndpointConnectionStatus::Connected,
                "attach_existing_runtime",
            ),
            (
                RuntimeEndpointConnectionStatus::Missing,
                "start_missing_runtime",
            ),
            (
                RuntimeEndpointConnectionStatus::Stale,
                "recover_stale_runtime",
            ),
            (
                RuntimeEndpointConnectionStatus::Unavailable,
                "fail_unavailable_endpoint",
            ),
        ];

        for (endpoint_connection, expected_path) in cases {
            let decision =
                SessionAcquisitionDecision::from(session_context_with(endpoint_connection));

            assert_eq!(decision.path_name(), expected_path);
        }

        #[cfg(not(unix))]
        {
            let decision = SessionAcquisitionDecision::from(session_context_with(
                RuntimeEndpointConnectionStatus::UnsupportedTransport,
            ));

            assert_eq!(decision.path_name(), "fail_unsupported_transport");
        }
    }

    #[test]
    fn selects_runtime_startup_decision_from_endpoint_connection() {
        let cases = [
            (
                RuntimeEndpointConnectionStatus::Connected,
                "attach_started_runtime",
            ),
            (
                RuntimeEndpointConnectionStatus::Missing,
                "launch_missing_runtime",
            ),
            (
                RuntimeEndpointConnectionStatus::Stale,
                "recover_stale_endpoint",
            ),
            (
                RuntimeEndpointConnectionStatus::Unavailable,
                "fail_unavailable_endpoint",
            ),
        ];

        for (endpoint_connection, expected_path) in cases {
            let decision = RuntimeStartupDecision::from(startup_context_with(endpoint_connection));

            assert_eq!(decision.path_name(), expected_path);
        }

        #[cfg(not(unix))]
        {
            let decision = RuntimeStartupDecision::from(startup_context_with(
                RuntimeEndpointConnectionStatus::UnsupportedTransport,
            ));

            assert_eq!(decision.path_name(), "fail_unsupported_transport");
        }
    }

    #[test]
    fn selected_decisions_keep_runtime_placement() {
        let context = session_context_with(RuntimeEndpointConnectionStatus::Missing);
        let expected_endpoint = context.placement.endpoint().path().to_path_buf();

        let decision = SessionAcquisitionDecision::from(context);
        assert_eq!(
            session_decision_endpoint_path(&decision),
            expected_endpoint.as_path()
        );

        let context = startup_context_with(RuntimeEndpointConnectionStatus::Missing);
        let expected_endpoint = context.placement.endpoint().path().to_path_buf();

        let decision = RuntimeStartupDecision::from(context);
        assert_eq!(
            startup_decision_endpoint_path(&decision),
            expected_endpoint.as_path()
        );
    }

    #[cfg(unix)]
    #[test]
    fn stale_endpoint_recovery_removes_socket_file() {
        let placement = placement();
        let endpoint = placement.endpoint().path().to_path_buf();
        std::fs::create_dir_all(endpoint.parent().unwrap()).unwrap();
        let listener = UnixListener::bind(&endpoint).unwrap();
        drop(listener);

        let recovered_placement = RuntimeStaleEndpointRecovery::new(placement)
            .recover()
            .unwrap();

        assert_eq!(recovered_placement.endpoint().path(), endpoint.as_path());
        assert!(!endpoint.exists());
    }

    #[cfg(unix)]
    #[test]
    fn stale_endpoint_recovery_refuses_non_socket_file() {
        let placement = placement();
        let endpoint = placement.endpoint().path().to_path_buf();
        std::fs::create_dir_all(endpoint.parent().unwrap()).unwrap();
        std::fs::write(&endpoint, "").unwrap();

        let error = RuntimeStaleEndpointRecovery::new(placement)
            .recover()
            .unwrap_err();

        assert!(error.to_string().contains("non-socket runtime endpoint"));
        assert!(endpoint.exists());
    }

    fn session_context_with(
        endpoint_connection: RuntimeEndpointConnectionStatus,
    ) -> SessionAcquisitionContext {
        SessionAcquisitionContext {
            placement: placement(),
            endpoint_connection,
        }
    }

    fn startup_context_with(
        endpoint_connection: RuntimeEndpointConnectionStatus,
    ) -> RuntimeStartupContext {
        RuntimeStartupContext {
            placement: placement(),
            endpoint_connection,
        }
    }

    fn placement() -> RuntimePlacement {
        let tmp = tempfile::tempdir().unwrap();
        let instance =
            RuntimeInstance::from_database(&RuntimeDatabase::sqlite(tmp.path().join("synd.db")))
                .unwrap();
        RuntimePlacement::from_instance(RuntimeRoot::from(tmp.path().join("runtime")), instance)
    }

    fn session_decision_endpoint_path(decision: &SessionAcquisitionDecision) -> &Path {
        match decision {
            SessionAcquisitionDecision::AttachExistingRuntime { placement }
            | SessionAcquisitionDecision::StartMissingRuntime { placement }
            | SessionAcquisitionDecision::RecoverStaleRuntime { placement }
            | SessionAcquisitionDecision::FailUnavailableEndpoint { placement } => {
                placement.endpoint().path()
            }
            #[cfg(not(unix))]
            SessionAcquisitionDecision::FailUnsupportedTransport { placement } => {
                placement.endpoint().path()
            }
        }
    }

    fn startup_decision_endpoint_path(decision: &RuntimeStartupDecision) -> &Path {
        match decision {
            RuntimeStartupDecision::AttachStartedRuntime { placement }
            | RuntimeStartupDecision::LaunchMissingRuntime { placement }
            | RuntimeStartupDecision::RecoverStaleEndpoint { placement }
            | RuntimeStartupDecision::FailUnavailableEndpoint { placement } => {
                placement.endpoint().path()
            }
            #[cfg(not(unix))]
            RuntimeStartupDecision::FailUnsupportedTransport { placement } => {
                placement.endpoint().path()
            }
        }
    }
}
