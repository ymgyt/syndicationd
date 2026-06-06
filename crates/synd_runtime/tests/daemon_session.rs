#![cfg(unix)]

use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use synd_runtime::{
    Daemon, DaemonConfig, DaemonState, Error, Runtime, RuntimeConfig, RuntimeDatabase,
};
use tokio::{task::JoinHandle, time::sleep};

mod daemon_session {
    mod two_clients {
        use crate::DaemonSessionHarness;
        use synd_protocol::{
            capability,
            session::{CloseSessionRequest, OpenSessionRequest, RenewSessionRequest},
        };

        #[tokio::test]
        async fn waits_for_last_close() -> synd_runtime::Result<()> {
            let harness = DaemonSessionHarness::start().await?;
            let first = harness.first_runtime().acquire_session().await?;
            let second = harness.second_runtime().acquire_session().await?;
            assert_eq!(
                first.available_capabilities(),
                &capability::local_api_capabilities()
            );
            assert_eq!(
                second.available_capabilities(),
                &capability::local_api_capabilities()
            );
            harness.assert_active_sessions(2).await?;

            let probe = second
                .client()
                .open_session(OpenSessionRequest::new(capability::local_api_capabilities()))
                .await?;
            assert_eq!(
                probe.available_capabilities(),
                &capability::local_api_capabilities()
            );
            assert_eq!(
                probe.lease().duration(),
                DaemonSessionHarness::LEASE_DURATION
            );
            harness.assert_active_sessions(3).await?;

            let renewed = second
                .client()
                .renew_session(RenewSessionRequest::new(probe.session_id().clone()))
                .await?;
            assert_eq!(renewed.session_id(), probe.session_id());
            assert_eq!(
                renewed.lease().duration(),
                DaemonSessionHarness::LEASE_DURATION
            );

            second
                .client()
                .close_session(CloseSessionRequest::new(probe.session_id().clone()))
                .await?;
            harness.assert_active_sessions(2).await?;
            first.close().await?;
            harness.assert_active_sessions(1).await?;
            harness.assert_running_after_idle_grace().await?;

            second.close().await?;
            harness.wait_until_stopped().await?;
            harness.finish().await
        }
    }
}

struct DaemonSessionHarness {
    _tempdir: tempfile::TempDir,
    first_runtime: Runtime,
    second_runtime: Runtime,
    endpoint: PathBuf,
    daemon: JoinHandle<synd_runtime::Result<()>>,
}

impl DaemonSessionHarness {
    const IDLE_SHUTDOWN_GRACE: Duration = Duration::from_millis(100);
    const LEASE_DURATION: Duration = Duration::from_secs(3);
    const POLL_INTERVAL: Duration = Duration::from_millis(25);
    const REQUEST_TIMEOUT: Duration = Duration::from_secs(2);
    const WAIT_TIMEOUT: Duration = Duration::from_secs(5);

    async fn start() -> synd_runtime::Result<Self> {
        let tempdir = tempfile::tempdir()?;
        let database = RuntimeDatabase::sqlite(tempdir.path().join("synd.db"));
        let runtime_root = tempdir.path().join("runtime");
        let first_runtime = Self::runtime(database.clone(), runtime_root.clone())?;
        let second_runtime = Self::runtime(database.clone(), runtime_root.clone())?;
        let endpoint = first_runtime
            .daemon()
            .inspect()
            .await?
            .placement()
            .endpoint()
            .into();
        let daemon = {
            let config = DaemonConfig::new(database)
                .with_runtime_root(runtime_root)
                .with_session_lease_duration(Self::LEASE_DURATION)
                .with_session_idle_shutdown_grace(Self::IDLE_SHUTDOWN_GRACE);

            tokio::spawn(Daemon::new(config).serve())
        };
        let harness = Self {
            _tempdir: tempdir,
            first_runtime,
            second_runtime,
            endpoint,
            daemon,
        };

        harness.wait_until_running().await?;
        Ok(harness)
    }

    fn runtime(database: RuntimeDatabase, runtime_root: PathBuf) -> synd_runtime::Result<Runtime> {
        let config = RuntimeConfig::new(database)
            .with_runtime_root(runtime_root)
            .with_api_timeout(Self::REQUEST_TIMEOUT, "synd-runtime-daemon-session-test")
            .with_session_timeout(Self::WAIT_TIMEOUT);

        Runtime::try_new(config)
    }

    fn first_runtime(&self) -> &Runtime {
        &self.first_runtime
    }

    fn second_runtime(&self) -> &Runtime {
        &self.second_runtime
    }

    async fn wait_until_running(&self) -> synd_runtime::Result<()> {
        self.wait_for(
            DaemonState::Running,
            Error::EndpointReadyTimeout {
                endpoint: self.endpoint.clone(),
            },
        )
        .await
    }

    async fn wait_until_stopped(&self) -> synd_runtime::Result<()> {
        self.wait_for(
            DaemonState::NotRunning,
            Error::EndpointStopTimeout {
                endpoint: self.endpoint.clone(),
            },
        )
        .await
    }

    async fn assert_running_after_idle_grace(&self) -> synd_runtime::Result<()> {
        sleep(Self::IDLE_SHUTDOWN_GRACE * 2).await;

        let status = self.first_runtime.daemon().inspect().await?;
        assert_eq!(status.state(), DaemonState::Running);
        Ok(())
    }

    async fn assert_active_sessions(&self, expected: usize) -> synd_runtime::Result<()> {
        let status = self.first_runtime.daemon().inspect().await?;
        let sessions = status
            .sessions()
            .expect("running daemon should report session status");
        let idle_shutdown = sessions.idle_shutdown();

        assert_eq!(status.state(), DaemonState::Running);
        assert_eq!(sessions.active_sessions(), expected);
        assert_eq!(sessions.lease_duration(), Self::LEASE_DURATION);
        assert!(idle_shutdown.is_enabled());
        assert_eq!(idle_shutdown.grace(), Some(Self::IDLE_SHUTDOWN_GRACE));
        assert!(!idle_shutdown.is_pending());
        Ok(())
    }

    async fn wait_for(
        &self,
        expected_state: DaemonState,
        timeout_error: Error,
    ) -> synd_runtime::Result<()> {
        let deadline = Instant::now() + Self::WAIT_TIMEOUT;

        loop {
            let status = self.first_runtime.daemon().inspect().await?;
            if status.state() == expected_state {
                return Ok(());
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(timeout_error);
            }

            sleep(Self::POLL_INTERVAL.min(deadline - now)).await;
        }
    }

    async fn finish(self) -> synd_runtime::Result<()> {
        let daemon = tokio::time::timeout(Self::WAIT_TIMEOUT, self.daemon)
            .await
            .expect("daemon serve task did not finish after endpoint stopped");

        daemon.expect("daemon serve task panicked")
    }
}
