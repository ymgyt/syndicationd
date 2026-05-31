use std::time::{Duration, Instant};

use tokio::time::sleep;

use crate::{
    DaemonState, DaemonStatus, Error, Result, Runtime, RuntimePlacementSummary, ShutdownResult,
    connection::{RuntimeEndpointConnectionStatus, RuntimeEndpointConnector},
    placement::RuntimePlacement,
};

const SHUTDOWN_WAIT_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy)]
pub struct Control<'a> {
    runtime: &'a Runtime,
}

impl<'a> Control<'a> {
    pub(crate) fn new(runtime: &'a Runtime) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &Runtime {
        self.runtime
    }

    pub async fn inspect(&self) -> Result<DaemonStatus> {
        let context = self.resolve_context().await?;

        context.status()
    }

    pub async fn shutdown(&self) -> Result<ShutdownResult> {
        let context = self.resolve_context().await?;
        let decision = DaemonShutdownDecision::from(context);

        match decision {
            DaemonShutdownDecision::RequestShutdown { placement } => {
                let summary = RuntimePlacementSummary::from_placement(&placement);
                DaemonControlClient::new(self.runtime, &placement)?
                    .shutdown()
                    .await?;
                DaemonShutdownWaiter::new(
                    placement,
                    self.runtime.config().session().acquire_timeout(),
                )
                .wait()
                .await?;

                Ok(ShutdownResult::new(DaemonStatus::new(
                    DaemonState::NotRunning,
                    summary,
                )))
            }
            DaemonShutdownDecision::AlreadyStopped { placement } => {
                Ok(ShutdownResult::new(DaemonStatus::new(
                    DaemonState::NotRunning,
                    RuntimePlacementSummary::from_placement(&placement),
                )))
            }
            DaemonShutdownDecision::FailUnavailableEndpoint { .. } => {
                Err(Error::NotImplemented("daemon endpoint unavailable"))
            }
            #[cfg(not(unix))]
            DaemonShutdownDecision::FailUnsupportedTransport { .. } => Err(Error::NotImplemented(
                "daemon control transport unsupported",
            )),
        }
    }

    #[expect(clippy::unused_async)]
    pub async fn restart(&self) -> Result<DaemonStatus> {
        Err(Error::NotImplemented("DaemonControl::restart"))
    }

    async fn resolve_context(&self) -> Result<DaemonControlContext> {
        let placement = self.runtime.placement().clone();
        let endpoint_connection = RuntimeEndpointConnector::new(placement.endpoint())
            .try_connect()
            .await;

        Ok(DaemonControlContext {
            placement,
            endpoint_connection,
        })
    }
}

/// Facts collected before selecting a daemon control action.
#[derive(Debug, Clone)]
struct DaemonControlContext {
    placement: RuntimePlacement,
    endpoint_connection: RuntimeEndpointConnectionStatus,
}

impl DaemonControlContext {
    fn status(&self) -> Result<DaemonStatus> {
        match self.endpoint_connection {
            RuntimeEndpointConnectionStatus::Connected => Ok(DaemonStatus::new(
                DaemonState::Running,
                RuntimePlacementSummary::from_placement(&self.placement),
            )),
            RuntimeEndpointConnectionStatus::Missing | RuntimeEndpointConnectionStatus::Stale => {
                Ok(DaemonStatus::new(
                    DaemonState::NotRunning,
                    RuntimePlacementSummary::from_placement(&self.placement),
                ))
            }
            RuntimeEndpointConnectionStatus::Unavailable => {
                Err(Error::NotImplemented("daemon endpoint unavailable"))
            }
            #[cfg(not(unix))]
            RuntimeEndpointConnectionStatus::UnsupportedTransport => Err(Error::NotImplemented(
                "daemon control transport unsupported",
            )),
        }
    }
}

/// Branch selected for daemon shutdown.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonShutdownDecision {
    RequestShutdown {
        placement: RuntimePlacement,
    },
    AlreadyStopped {
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

impl From<DaemonControlContext> for DaemonShutdownDecision {
    fn from(context: DaemonControlContext) -> Self {
        match context.endpoint_connection {
            RuntimeEndpointConnectionStatus::Connected => Self::RequestShutdown {
                placement: context.placement,
            },
            RuntimeEndpointConnectionStatus::Missing | RuntimeEndpointConnectionStatus::Stale => {
                Self::AlreadyStopped {
                    placement: context.placement,
                }
            }
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

/// Sends daemon control requests over the resolved runtime endpoint.
struct DaemonControlClient {
    client: synd_client::Client,
}

impl DaemonControlClient {
    #[cfg(unix)]
    fn new(runtime: &Runtime, placement: &RuntimePlacement) -> Result<Self> {
        let client = synd_client::Client::new_unix(
            placement.endpoint().path(),
            synd_client::ClientOptions::new(
                runtime.config().client().request_timeout(),
                runtime.config().client().user_agent(),
            ),
        )?;

        Ok(Self { client })
    }

    #[cfg(not(unix))]
    fn new(runtime: &Runtime, placement: &RuntimePlacement) -> Result<Self> {
        let _ = (runtime, placement);

        Err(Error::NotImplemented(
            "daemon control transport unsupported",
        ))
    }

    async fn shutdown(&self) -> Result<()> {
        self.client.shutdown_daemon().await?;
        Ok(())
    }
}

/// Waits until the daemon endpoint stops accepting connections.
struct DaemonShutdownWaiter {
    placement: RuntimePlacement,
    timeout: Duration,
}

impl DaemonShutdownWaiter {
    fn new(placement: RuntimePlacement, timeout: Duration) -> Self {
        Self { placement, timeout }
    }

    async fn wait(&self) -> Result<()> {
        let deadline = Instant::now() + self.timeout;

        loop {
            let endpoint_connection = RuntimeEndpointConnector::new(self.placement.endpoint())
                .try_connect()
                .await;
            match endpoint_connection {
                RuntimeEndpointConnectionStatus::Missing
                | RuntimeEndpointConnectionStatus::Stale => {
                    return Ok(());
                }
                RuntimeEndpointConnectionStatus::Connected => {}
                RuntimeEndpointConnectionStatus::Unavailable => {
                    return Err(Error::NotImplemented("daemon endpoint unavailable"));
                }
                #[cfg(not(unix))]
                RuntimeEndpointConnectionStatus::UnsupportedTransport => {
                    return Err(Error::NotImplemented(
                        "daemon control transport unsupported",
                    ));
                }
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(anyhow::anyhow!(
                    "timed out waiting for daemon endpoint {} to stop",
                    self.placement.endpoint().path().display()
                )
                .into());
            }

            sleep(SHUTDOWN_WAIT_INTERVAL.min(deadline - now)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        RuntimeDatabase,
        connection::RuntimeEndpointConnectionStatus,
        instance::RuntimeInstance,
        placement::{RuntimePlacement, RuntimeRoot},
    };

    use super::{DaemonControlContext, DaemonShutdownDecision};

    #[test]
    fn resolves_daemon_status_from_endpoint_connection() {
        let cases = [
            (
                RuntimeEndpointConnectionStatus::Connected,
                crate::DaemonState::Running,
            ),
            (
                RuntimeEndpointConnectionStatus::Missing,
                crate::DaemonState::NotRunning,
            ),
            (
                RuntimeEndpointConnectionStatus::Stale,
                crate::DaemonState::NotRunning,
            ),
        ];

        for (endpoint_connection, expected_state) in cases {
            let context = context_with(endpoint_connection);
            let expected_runtime_instance_id = context.placement.instance().id().to_string();
            let expected_database = context
                .placement
                .instance()
                .canonical_database_path()
                .to_path_buf();
            let expected_endpoint = context.placement.endpoint().path().to_path_buf();
            let status = context.status().unwrap();

            assert_eq!(status.state(), expected_state);
            assert_eq!(
                status.placement().runtime_instance_id(),
                expected_runtime_instance_id
            );
            assert_eq!(status.placement().database(), expected_database.as_path());
            assert_eq!(status.placement().endpoint(), expected_endpoint.as_path());
        }
    }

    #[test]
    fn selects_shutdown_decision_from_endpoint_connection() {
        let cases = [
            (
                RuntimeEndpointConnectionStatus::Connected,
                "request_shutdown",
            ),
            (RuntimeEndpointConnectionStatus::Missing, "already_stopped"),
            (RuntimeEndpointConnectionStatus::Stale, "already_stopped"),
            (
                RuntimeEndpointConnectionStatus::Unavailable,
                "fail_unavailable_endpoint",
            ),
        ];

        for (endpoint_connection, expected_path) in cases {
            let decision = DaemonShutdownDecision::from(context_with(endpoint_connection));

            assert_eq!(shutdown_decision_path(&decision), expected_path);
        }

        #[cfg(not(unix))]
        {
            let decision = DaemonShutdownDecision::from(context_with(
                RuntimeEndpointConnectionStatus::UnsupportedTransport,
            ));

            assert_eq!(
                shutdown_decision_path(&decision),
                "fail_unsupported_transport"
            );
        }
    }

    fn context_with(endpoint_connection: RuntimeEndpointConnectionStatus) -> DaemonControlContext {
        DaemonControlContext {
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

    fn shutdown_decision_path(decision: &DaemonShutdownDecision) -> &'static str {
        match decision {
            DaemonShutdownDecision::RequestShutdown { .. } => "request_shutdown",
            DaemonShutdownDecision::AlreadyStopped { .. } => "already_stopped",
            DaemonShutdownDecision::FailUnavailableEndpoint { .. } => "fail_unavailable_endpoint",
            #[cfg(not(unix))]
            DaemonShutdownDecision::FailUnsupportedTransport { .. } => "fail_unsupported_transport",
        }
    }
}
