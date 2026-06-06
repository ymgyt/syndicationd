use std::time::{Duration, Instant};

#[cfg(unix)]
use rustix::process::Signal;
use synd_protocol::daemon::DaemonStatusResponse;
use tokio::time::sleep;

#[cfg(unix)]
use crate::daemon::{
    DaemonClaim, DaemonClaimLockAcquirer, SignalTarget, remove_stale_claim,
    wait_until_claim_released,
};
use crate::{
    DaemonState, DaemonStatus, Error, PlacementSummary, Result, Runtime, ShutdownResult,
    connection::{RuntimeEndpointConnectionStatus, RuntimeEndpointConnector},
    placement::PlacementSpec,
    startup::{StartupLock, StartupLockAcquirer, StartupLockAcquisition},
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
        let decision = DaemonStatusDecision::from(context);

        match decision {
            DaemonStatusDecision::RequestStatus { placement } => {
                let summary = PlacementSummary::from_placement(&placement);
                match DaemonControlClient::new(self.runtime, &placement)?
                    .status()
                    .await
                {
                    Ok(status) => Ok(DaemonStatus::running(summary, status.sessions().clone())),
                    Err(error) if daemon_status_endpoint_missing(&error) => {
                        Ok(DaemonStatus::new(DaemonState::Running, summary))
                    }
                    Err(error) => Err(error.into()),
                }
            }
            DaemonStatusDecision::AlreadyStopped { placement } => Ok(DaemonStatus::new(
                DaemonState::NotRunning,
                PlacementSummary::from_placement(&placement),
            )),
            DaemonStatusDecision::FailUnavailableEndpoint { placement } => {
                Err(Error::EndpointUnavailable {
                    context: "daemon endpoint",
                    endpoint: placement.endpoint().path().to_path_buf(),
                })
            }
            #[cfg(not(unix))]
            DaemonStatusDecision::FailUnsupportedTransport { .. } => {
                Err(Error::UnsupportedTransport {
                    context: "daemon control transport",
                })
            }
        }
    }

    pub async fn shutdown(&self) -> Result<ShutdownResult> {
        let context = self.resolve_context().await?;
        let decision = DaemonShutdownDecision::from(context);

        match decision {
            DaemonShutdownDecision::RequestShutdown { placement } => {
                let summary = PlacementSummary::from_placement(&placement);
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
                    PlacementSummary::from_placement(&placement),
                )))
            }
            DaemonShutdownDecision::FailUnavailableEndpoint { placement } => {
                Err(Error::EndpointUnavailable {
                    context: "daemon endpoint",
                    endpoint: placement.endpoint().path().to_path_buf(),
                })
            }
            #[cfg(not(unix))]
            DaemonShutdownDecision::FailUnsupportedTransport { .. } => {
                Err(Error::UnsupportedTransport {
                    context: "daemon control transport",
                })
            }
        }
    }

    pub async fn force_shutdown(&self) -> Result<ShutdownResult> {
        #[cfg(unix)]
        {
            return self.force_shutdown_unix().await;
        }

        #[cfg(not(unix))]
        {
            Err(Error::UnsupportedTransport {
                context: "daemon control transport",
            })
        }
    }

    #[cfg(unix)]
    async fn force_shutdown_unix(&self) -> Result<ShutdownResult> {
        let placement = self.runtime.placement().clone();
        let summary = PlacementSummary::from_placement(&placement);
        let _startup_lock = acquire_startup_lock(&placement)?;

        let Some(claim) = DaemonClaim::read(placement.daemon_claim_path())? else {
            return self.shutdown_without_claim(placement, summary).await;
        };

        let claim_lock = DaemonClaimLockAcquirer::new(placement.daemon_claim_lock_path());
        if !claim_lock.is_held()? {
            return self
                .shutdown_with_stale_claim(placement, summary, "daemon claim lock is not held")
                .await;
        }

        let target = SignalTarget::validate(&placement, &claim)?;
        if !target.send(Signal::TERM)? {
            remove_stale_claim(placement.daemon_claim_path())?;
            return Ok(ShutdownResult::new(DaemonStatus::new(
                DaemonState::NotRunning,
                summary,
            )));
        }

        let timeout = self.runtime.config().session().acquire_timeout();
        if !wait_until_claim_released(placement.daemon_claim_lock_path(), timeout).await? {
            let target = SignalTarget::validate(&placement, &claim)?;
            let _ = target.send(Signal::KILL)?;
            if !wait_until_claim_released(placement.daemon_claim_lock_path(), timeout).await? {
                return Err(Error::ForceShutdownTimeout { pid: claim.pid() });
            }
        }

        remove_stale_claim(placement.daemon_claim_path())?;

        Ok(ShutdownResult::new(DaemonStatus::new(
            DaemonState::NotRunning,
            summary,
        )))
    }

    #[cfg(unix)]
    async fn shutdown_without_claim(
        &self,
        placement: PlacementSpec,
        summary: PlacementSummary,
    ) -> Result<ShutdownResult> {
        match RuntimeEndpointConnector::new(placement.endpoint())
            .try_connect()
            .await
        {
            RuntimeEndpointConnectionStatus::Missing | RuntimeEndpointConnectionStatus::Stale => {
                Ok(ShutdownResult::new(DaemonStatus::new(
                    DaemonState::NotRunning,
                    summary,
                )))
            }
            RuntimeEndpointConnectionStatus::Connected
            | RuntimeEndpointConnectionStatus::Unavailable => Err(Error::ForceShutdownRefused {
                reason: format!(
                    "daemon claim is missing at {}; cannot prove endpoint owner",
                    placement.daemon_claim_path().path().display()
                ),
            }),
            #[cfg(not(unix))]
            RuntimeEndpointConnectionStatus::UnsupportedTransport => {
                Err(Error::UnsupportedTransport {
                    context: "daemon control transport",
                })
            }
        }
    }

    #[cfg(unix)]
    async fn shutdown_with_stale_claim(
        &self,
        placement: PlacementSpec,
        summary: PlacementSummary,
        reason: &'static str,
    ) -> Result<ShutdownResult> {
        match RuntimeEndpointConnector::new(placement.endpoint())
            .try_connect()
            .await
        {
            RuntimeEndpointConnectionStatus::Missing | RuntimeEndpointConnectionStatus::Stale => {
                remove_stale_claim(placement.daemon_claim_path())?;
                Ok(ShutdownResult::new(DaemonStatus::new(
                    DaemonState::NotRunning,
                    summary,
                )))
            }
            RuntimeEndpointConnectionStatus::Connected
            | RuntimeEndpointConnectionStatus::Unavailable => Err(Error::ForceShutdownRefused {
                reason: format!(
                    "{reason}; refusing to use stale daemon claim while endpoint {} is not stopped",
                    placement.endpoint().path().display()
                ),
            }),
            #[cfg(not(unix))]
            RuntimeEndpointConnectionStatus::UnsupportedTransport => {
                Err(Error::UnsupportedTransport {
                    context: "daemon control transport",
                })
            }
        }
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

#[cfg(unix)]
fn acquire_startup_lock(placement: &PlacementSpec) -> Result<StartupLock> {
    match StartupLockAcquirer::new(placement.startup_lock_path()).try_acquire()? {
        StartupLockAcquisition::Acquired(lock) => Ok(lock),
        StartupLockAcquisition::AlreadyHeld => Err(Error::ForceShutdownRefused {
            reason: format!(
                "startup lock is already held at {}",
                placement.startup_lock_path().path().display()
            ),
        }),
        #[cfg(not(unix))]
        StartupLockAcquisition::UnsupportedTransport => Err(Error::UnsupportedTransport {
            context: "daemon startup lock",
        }),
    }
}

/// Facts collected before selecting a daemon control action.
#[derive(Debug, Clone)]
struct DaemonControlContext {
    placement: PlacementSpec,
    endpoint_connection: RuntimeEndpointConnectionStatus,
}

/// Branch selected for daemon status inspection.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonStatusDecision {
    RequestStatus {
        placement: PlacementSpec,
    },
    AlreadyStopped {
        placement: PlacementSpec,
    },
    FailUnavailableEndpoint {
        placement: PlacementSpec,
    },
    #[cfg(not(unix))]
    FailUnsupportedTransport {
        placement: PlacementSpec,
    },
}

impl From<DaemonControlContext> for DaemonStatusDecision {
    fn from(context: DaemonControlContext) -> Self {
        match context.endpoint_connection {
            RuntimeEndpointConnectionStatus::Connected => Self::RequestStatus {
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

/// Branch selected for daemon shutdown.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DaemonShutdownDecision {
    RequestShutdown {
        placement: PlacementSpec,
    },
    AlreadyStopped {
        placement: PlacementSpec,
    },
    FailUnavailableEndpoint {
        placement: PlacementSpec,
    },
    #[cfg(not(unix))]
    FailUnsupportedTransport {
        placement: PlacementSpec,
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
    fn new(runtime: &Runtime, placement: &PlacementSpec) -> Result<Self> {
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
    fn new(runtime: &Runtime, placement: &PlacementSpec) -> Result<Self> {
        let _ = (runtime, placement);

        Err(Error::UnsupportedTransport {
            context: "daemon control transport",
        })
    }

    async fn shutdown(&self) -> Result<()> {
        self.client.shutdown_daemon().await?;
        Ok(())
    }

    async fn status(&self) -> std::result::Result<DaemonStatusResponse, synd_client::SyndApiError> {
        self.client.daemon_status().await
    }
}

fn daemon_status_endpoint_missing(error: &synd_client::SyndApiError) -> bool {
    matches!(
        error,
        synd_client::SyndApiError::HttpStatus {
            status,
            url: Some(url),
        } if status.as_u16() == 404 && url.path() == synd_protocol::daemon::STATUS_PATH
    )
}

/// Waits until the daemon endpoint stops accepting connections.
struct DaemonShutdownWaiter {
    placement: PlacementSpec,
    timeout: Duration,
}

impl DaemonShutdownWaiter {
    fn new(placement: PlacementSpec, timeout: Duration) -> Self {
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
                    return Err(Error::EndpointUnavailable {
                        context: "daemon endpoint",
                        endpoint: self.placement.endpoint().path().to_path_buf(),
                    });
                }
                #[cfg(not(unix))]
                RuntimeEndpointConnectionStatus::UnsupportedTransport => {
                    return Err(Error::UnsupportedTransport {
                        context: "daemon control transport",
                    });
                }
            }

            let now = Instant::now();
            if now >= deadline {
                return Err(Error::EndpointStopTimeout {
                    endpoint: self.placement.endpoint().path().to_path_buf(),
                });
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
        placement::{PlacementRoot, PlacementSpec},
    };

    use super::{DaemonControlContext, DaemonShutdownDecision, DaemonStatusDecision};

    mod status {
        use super::*;

        #[test]
        fn from_endpoint_connection() {
            let cases = [
                (RuntimeEndpointConnectionStatus::Connected, "request_status"),
                (RuntimeEndpointConnectionStatus::Missing, "already_stopped"),
                (RuntimeEndpointConnectionStatus::Stale, "already_stopped"),
                (
                    RuntimeEndpointConnectionStatus::Unavailable,
                    "fail_unavailable_endpoint",
                ),
            ];

            for (endpoint_connection, expected_path) in cases {
                let decision = DaemonStatusDecision::from(context_with(endpoint_connection));

                assert_eq!(status_decision_path(&decision), expected_path);
            }

            #[cfg(not(unix))]
            {
                let decision = DaemonStatusDecision::from(context_with(
                    RuntimeEndpointConnectionStatus::UnsupportedTransport,
                ));

                assert_eq!(
                    status_decision_path(&decision),
                    "fail_unsupported_transport"
                );
            }
        }
    }

    mod shutdown_decision {
        use super::*;

        #[test]
        fn from_endpoint_connection() {
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
    }

    fn context_with(endpoint_connection: RuntimeEndpointConnectionStatus) -> DaemonControlContext {
        DaemonControlContext {
            placement: placement(),
            endpoint_connection,
        }
    }

    fn placement() -> PlacementSpec {
        let tmp = tempfile::tempdir().unwrap();
        let instance =
            RuntimeInstance::from_database(&RuntimeDatabase::sqlite(tmp.path().join("synd.db")))
                .unwrap();

        PlacementSpec::from_instance(PlacementRoot::from(tmp.path().join("runtime")), instance)
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

    fn status_decision_path(decision: &DaemonStatusDecision) -> &'static str {
        match decision {
            DaemonStatusDecision::RequestStatus { .. } => "request_status",
            DaemonStatusDecision::AlreadyStopped { .. } => "already_stopped",
            DaemonStatusDecision::FailUnavailableEndpoint { .. } => "fail_unavailable_endpoint",
            #[cfg(not(unix))]
            DaemonStatusDecision::FailUnsupportedTransport { .. } => "fail_unsupported_transport",
        }
    }
}
