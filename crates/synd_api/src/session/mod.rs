use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use synd_protocol::{
    CapabilitySet,
    daemon::{DaemonIdleShutdownStatus, DaemonSessionStatus},
    session::{
        CloseSessionErrorResponse, CloseSessionRequest, CloseSessionResponse,
        OpenSessionErrorResponse, OpenSessionRequest, OpenSessionResponse,
        RenewSessionErrorResponse, RenewSessionRequest, RenewSessionResponse, SessionLease,
    },
};
use tracing::debug;

use crate::shutdown::Shutdown;

mod id_issuer;
mod idle_shutdown;
mod state;
mod sweeper;

pub use id_issuer::SessionIdIssuer;
pub(crate) use sweeper::DaemonSessionSweeper;

use idle_shutdown::SessionIdleShutdown;
use state::{DaemonSessionsState, SessionLeaseDeadline, SessionRenewal, SessionSweep};

pub const DEFAULT_DAEMON_IDLE_SHUTDOWN_GRACE: Duration = Duration::from_secs(30);
pub const DEFAULT_DAEMON_SESSION_LEASE_DURATION: Duration = SessionLease::DEFAULT_DURATION;
pub const DEFAULT_DAEMON_SESSION_SWEEP_INTERVAL: Duration = Duration::from_secs(5);

/// Session lifecycle settings used by a daemon API service.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonSessionConfig {
    lease_policy: DaemonSessionLeasePolicy,
    idle_shutdown_grace: Duration,
}

impl DaemonSessionConfig {
    #[must_use]
    pub fn new(lease_policy: DaemonSessionLeasePolicy, idle_shutdown_grace: Duration) -> Self {
        Self {
            lease_policy,
            idle_shutdown_grace,
        }
    }

    #[must_use]
    pub fn with_lease_duration(self, lease_duration: Duration) -> Self {
        self.with_lease_policy(DaemonSessionLeasePolicy::new(
            lease_duration,
            self.lease_policy.sweep_interval(),
        ))
    }

    #[must_use]
    pub fn with_lease_policy(self, lease_policy: DaemonSessionLeasePolicy) -> Self {
        Self {
            lease_policy,
            ..self
        }
    }

    #[must_use]
    pub fn with_idle_shutdown_grace(self, idle_shutdown_grace: Duration) -> Self {
        Self {
            idle_shutdown_grace,
            ..self
        }
    }

    #[must_use]
    pub fn lease_policy(self) -> DaemonSessionLeasePolicy {
        self.lease_policy
    }

    #[must_use]
    pub fn idle_shutdown_grace(self) -> Duration {
        self.idle_shutdown_grace
    }
}

impl Default for DaemonSessionConfig {
    fn default() -> Self {
        Self::new(
            DaemonSessionLeasePolicy::default(),
            DEFAULT_DAEMON_IDLE_SHUTDOWN_GRACE,
        )
    }
}

/// Owns daemon sessions accepted by this API process.
#[derive(Debug, Clone)]
pub struct DaemonSessions {
    inner: Arc<DaemonSessionsInner>,
}

impl DaemonSessions {
    /// Creates an empty registry for one daemon run. A shutdown handle enables idle shutdown.
    pub fn new(
        supported_capabilities: CapabilitySet,
        config: DaemonSessionConfig,
        id_issuer: SessionIdIssuer,
        shutdown: Option<Shutdown>,
    ) -> Self {
        Self {
            inner: Arc::new(DaemonSessionsInner {
                supported_capabilities,
                lease_policy: config.lease_policy(),
                id_issuer,
                idle_shutdown: shutdown.map(|shutdown| {
                    SessionIdleShutdown::new(config.idle_shutdown_grace(), shutdown)
                }),
                state: Mutex::new(DaemonSessionsState::default()),
            }),
        }
    }

    pub fn open(
        &self,
        request: &OpenSessionRequest,
    ) -> Result<OpenSessionResponse, OpenSessionErrorResponse> {
        self.open_at(request, Instant::now())
    }

    fn open_at(
        &self,
        request: &OpenSessionRequest,
        now: Instant,
    ) -> Result<OpenSessionResponse, OpenSessionErrorResponse> {
        let supported = &self.inner.supported_capabilities;
        let missing_capabilities = request.required_capabilities().missing_from(supported);
        if !missing_capabilities.is_empty() {
            debug!(?missing_capabilities, "Rejected daemon session open");
            return Err(OpenSessionErrorResponse::from_missing_capabilities(
                missing_capabilities,
            ));
        }

        let session_id = self.inner.id_issuer.issue();
        let lease = self.inner.lease_policy.lease();
        let lease_deadline = self.inner.lease_policy.deadline_from(now);
        let active_sessions = self.update_sessions(|state| {
            state.insert(session_id.clone(), lease_deadline);
            state.active_session_count()
        });
        debug!(
            %session_id,
            active_sessions,
            lease_duration_ms = lease.duration().as_millis(),
            "Opened daemon session"
        );

        Ok(OpenSessionResponse::with_lease(
            session_id,
            supported.clone(),
            lease,
        ))
    }

    pub fn renew(
        &self,
        request: &RenewSessionRequest,
    ) -> Result<RenewSessionResponse, RenewSessionErrorResponse> {
        self.renew_at(request, Instant::now())
    }

    fn renew_at(
        &self,
        request: &RenewSessionRequest,
        now: Instant,
    ) -> Result<RenewSessionResponse, RenewSessionErrorResponse> {
        let session_id = request.session_id().clone();
        let lease_deadline = self.inner.lease_policy.deadline_from(now);
        let renewal = self.update_sessions(|state| state.renew(&session_id, now, lease_deadline));

        match renewal {
            SessionRenewal::Renewed => {
                debug!(
                    %session_id,
                    lease_expires_in_ms = lease_deadline.remaining_from(now).as_millis(),
                    "Renewed daemon session"
                );
                Ok(RenewSessionResponse::new(
                    session_id,
                    self.inner.lease_policy.lease(),
                ))
            }
            SessionRenewal::Unknown => {
                debug!(%session_id, "Rejected daemon session renew");
                Err(RenewSessionErrorResponse::unknown_session(session_id))
            }
            SessionRenewal::Expired => {
                debug!(%session_id, "Expired daemon session during renew");
                Err(RenewSessionErrorResponse::unknown_session(session_id))
            }
        }
    }

    pub fn close(
        &self,
        request: &CloseSessionRequest,
    ) -> Result<CloseSessionResponse, CloseSessionErrorResponse> {
        let session_id = request.session_id();
        if !self.update_sessions(|state| state.remove(session_id)) {
            debug!(%session_id, "Rejected daemon session close");
            return Err(CloseSessionErrorResponse::unknown_session(
                session_id.clone(),
            ));
        }

        debug!(%session_id, "Closed daemon session");
        Ok(CloseSessionResponse::new())
    }

    fn sweep_interval(&self) -> Duration {
        self.inner.lease_policy.sweep_interval()
    }

    fn sweep_expired(&self) -> SessionSweep {
        self.sweep_expired_at(Instant::now())
    }

    fn sweep_expired_at(&self, now: Instant) -> SessionSweep {
        let sweep = self.update_sessions(|state| state.sweep_expired(now));
        if !sweep.expired_sessions.is_empty() {
            debug!(
                expired_sessions = ?sweep.expired_sessions,
                active_sessions = sweep.active_sessions,
                "Expired daemon sessions during sweep"
            );
        }
        sweep
    }

    pub fn status(&self) -> DaemonSessionStatus {
        let state = self.lock_state();
        let idle_shutdown = match &self.inner.idle_shutdown {
            Some(idle_shutdown) => DaemonIdleShutdownStatus::enabled(
                idle_shutdown.grace(),
                state.idle_shutdown_pending(),
            ),
            None => DaemonIdleShutdownStatus::disabled(),
        };

        DaemonSessionStatus::new(
            state.active_session_count(),
            self.inner.lease_policy.lease_duration(),
            self.inner.lease_policy.sweep_interval(),
            idle_shutdown,
        )
    }

    fn update_sessions<T>(&self, update: impl FnOnce(&mut DaemonSessionsState) -> T) -> T {
        let mut state = self.lock_state();
        let was_empty = state.active_session_count() == 0;
        let result = update(&mut state);
        let is_empty = state.active_session_count() == 0;
        if was_empty != is_empty {
            state.idle_shutdown_timer = if is_empty {
                self.inner
                    .idle_shutdown
                    .as_ref()
                    .map(|idle| idle.schedule_check(self))
            } else {
                None
            };
        }
        result
    }

    fn lock_state(&self) -> MutexGuard<'_, DaemonSessionsState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[cfg(test)]
    fn active_session_count(&self) -> usize {
        self.lock_state().active_session_count()
    }
}

#[derive(Debug)]
struct DaemonSessionsInner {
    supported_capabilities: CapabilitySet,
    lease_policy: DaemonSessionLeasePolicy,
    id_issuer: SessionIdIssuer,
    idle_shutdown: Option<SessionIdleShutdown>,
    state: Mutex<DaemonSessionsState>,
}

/// Policy used to grant leases and expire sessions that stop renewing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonSessionLeasePolicy {
    lease_duration: Duration,
    sweep_interval: Duration,
}

impl DaemonSessionLeasePolicy {
    #[must_use]
    pub fn new(lease_duration: Duration, sweep_interval: Duration) -> Self {
        Self {
            lease_duration,
            sweep_interval,
        }
    }

    #[must_use]
    fn lease(self) -> SessionLease {
        SessionLease::new(self.lease_duration)
    }

    #[must_use]
    fn deadline_from(self, now: Instant) -> SessionLeaseDeadline {
        SessionLeaseDeadline::new(now + self.lease_duration)
    }

    #[must_use]
    pub fn lease_duration(self) -> Duration {
        self.lease_duration
    }

    #[must_use]
    pub fn sweep_interval(self) -> Duration {
        self.sweep_interval
    }
}

impl Default for DaemonSessionLeasePolicy {
    fn default() -> Self {
        Self::new(
            DEFAULT_DAEMON_SESSION_LEASE_DURATION,
            DEFAULT_DAEMON_SESSION_SWEEP_INTERVAL,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::{Duration, Instant},
    };

    use synd_protocol::{
        CapabilitySet, capability,
        session::{
            CloseSessionErrorCode, CloseSessionRequest, OpenSessionRequest, RenewSessionErrorCode,
            RenewSessionRequest, SessionId,
        },
    };

    use crate::shutdown::Shutdown;

    use super::{DaemonSessionConfig, DaemonSessionLeasePolicy, DaemonSessions, SessionIdIssuer};

    fn test_sessions(config: DaemonSessionConfig, shutdown: Option<Shutdown>) -> DaemonSessions {
        DaemonSessions::new(
            capability::local_api_capabilities(),
            config,
            SessionIdIssuer::new(42, 1),
            shutdown,
        )
    }

    #[test]
    fn opens_and_closes_session() {
        let supported = CapabilitySet::new(["timeline.read", capability::SUBSCRIPTION_WRITE]);
        let sessions = DaemonSessions::new(
            supported.clone(),
            DaemonSessionConfig::default(),
            SessionIdIssuer::new(42, 1),
            None,
        );

        let opened = sessions
            .open(&OpenSessionRequest::new(CapabilitySet::new([
                "timeline.read",
            ])))
            .unwrap();

        assert_eq!(opened.available_capabilities(), &supported);
        assert_eq!(sessions.active_session_count(), 1);

        let close = CloseSessionRequest::new(opened.session_id().clone());
        sessions.close(&close).unwrap();
        assert_eq!(sessions.active_session_count(), 0);

        let error = sessions.close(&close).unwrap_err();
        assert_eq!(error.code(), CloseSessionErrorCode::UnknownSession);
        assert_eq!(error.session_id(), opened.session_id());
    }

    #[test]
    fn opens_session_with_lease_policy() {
        let now = Instant::now();
        let policy = DaemonSessionLeasePolicy::new(Duration::from_secs(12), Duration::from_secs(2));
        let config = DaemonSessionConfig::default().with_lease_policy(policy);
        let sessions = test_sessions(config, None);

        let opened = sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();

        assert_eq!(opened.lease().duration(), policy.lease_duration());
        assert_eq!(sessions.active_session_count(), 1);
    }

    #[test]
    fn rejects_session_when_required_capability_is_missing() {
        let sessions = test_sessions(DaemonSessionConfig::default(), None);

        let error = sessions
            .open(&OpenSessionRequest::new(CapabilitySet::new([
                capability::SUBSCRIPTION_WRITE,
                "unsupported.capability",
            ])))
            .unwrap_err();

        assert_eq!(
            error.missing_capabilities().names(),
            ["unsupported.capability"]
        );
        assert_eq!(sessions.active_session_count(), 0);
    }

    #[test]
    fn renews_session_before_lease_deadline() {
        let now = Instant::now();
        let policy = DaemonSessionLeasePolicy::new(Duration::from_secs(30), Duration::from_secs(5));
        let config = DaemonSessionConfig::default().with_lease_policy(policy);
        let sessions = test_sessions(config, None);
        let opened = sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();

        let renewed = sessions
            .renew_at(
                &RenewSessionRequest::new(opened.session_id().clone()),
                now + Duration::from_secs(10),
            )
            .unwrap();

        assert_eq!(renewed.session_id(), opened.session_id());
        assert_eq!(renewed.lease().duration(), policy.lease_duration());
        assert_eq!(sessions.active_session_count(), 1);
    }

    #[test]
    fn rejects_and_removes_expired_session_during_renew() {
        let now = Instant::now();
        let policy = DaemonSessionLeasePolicy::new(Duration::from_secs(30), Duration::from_secs(5));
        let config = DaemonSessionConfig::default().with_lease_policy(policy);
        let sessions = test_sessions(config, None);
        let opened = sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();

        let error = sessions
            .renew_at(
                &RenewSessionRequest::new(opened.session_id().clone()),
                now + Duration::from_secs(31),
            )
            .unwrap_err();

        assert_eq!(error.code(), RenewSessionErrorCode::UnknownSession);
        assert_eq!(error.session_id(), opened.session_id());
        assert_eq!(sessions.active_session_count(), 0);
    }

    #[test]
    fn rejects_unknown_session_during_renew() {
        let sessions = test_sessions(DaemonSessionConfig::default(), None);
        let session_id = SessionId::new("session-unknown");

        let error = sessions
            .renew(&RenewSessionRequest::new(session_id.clone()))
            .unwrap_err();

        assert_eq!(error.code(), RenewSessionErrorCode::UnknownSession);
        assert_eq!(error.session_id(), &session_id);
    }

    #[test]
    fn sweeps_expired_sessions() {
        let now = Instant::now();
        let policy = DaemonSessionLeasePolicy::new(Duration::from_secs(30), Duration::from_secs(5));
        let config = DaemonSessionConfig::default().with_lease_policy(policy);
        let sessions = test_sessions(config, None);
        let first = sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();
        sessions
            .open_at(
                &OpenSessionRequest::new(CapabilitySet::default()),
                now + Duration::from_secs(10),
            )
            .unwrap();

        let outcome = sessions.sweep_expired_at(now + Duration::from_secs(10));
        assert_eq!(outcome.expired_sessions, []);
        assert_eq!(outcome.active_sessions, 2);

        let outcome = sessions.sweep_expired_at(now + Duration::from_secs(31));
        assert_eq!(
            outcome.expired_sessions.as_slice(),
            std::slice::from_ref(first.session_id())
        );
        assert_eq!(outcome.active_sessions, 1);
        assert_eq!(sessions.active_session_count(), 1);
    }

    #[tokio::test]
    async fn schedules_idle_shutdown_after_last_session_closes() {
        let shutdown_called = Arc::new(AtomicBool::new(false));
        let shutdown_called_for_hook = Arc::clone(&shutdown_called);
        let shutdown = Shutdown::manual(move || {
            shutdown_called_for_hook.store(true, Ordering::Relaxed);
        });
        let config =
            DaemonSessionConfig::default().with_idle_shutdown_grace(Duration::from_millis(10));
        let sessions = test_sessions(config, Some(shutdown));
        let opened = sessions
            .open(&OpenSessionRequest::new(CapabilitySet::default()))
            .unwrap();

        sessions
            .close(&CloseSessionRequest::new(opened.session_id().clone()))
            .unwrap();

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if shutdown_called.load(Ordering::Relaxed) {
                    return;
                }

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn opening_session_cancels_pending_idle_shutdown() {
        let shutdown_called = Arc::new(AtomicBool::new(false));
        let shutdown_called_for_hook = Arc::clone(&shutdown_called);
        let shutdown = Shutdown::manual(move || {
            shutdown_called_for_hook.store(true, Ordering::Relaxed);
        });
        let config =
            DaemonSessionConfig::default().with_idle_shutdown_grace(Duration::from_millis(20));
        let sessions = test_sessions(config, Some(shutdown));
        let first = sessions
            .open(&OpenSessionRequest::new(CapabilitySet::default()))
            .unwrap();

        sessions
            .close(&CloseSessionRequest::new(first.session_id().clone()))
            .unwrap();
        let _second = sessions
            .open(&OpenSessionRequest::new(CapabilitySet::default()))
            .unwrap();
        tokio::time::sleep(Duration::from_millis(60)).await;

        assert!(!shutdown_called.load(Ordering::Relaxed));
    }

    #[tokio::test]
    async fn schedules_idle_shutdown_after_last_session_expires_during_sweep() {
        let now = Instant::now();
        let shutdown_called = Arc::new(AtomicBool::new(false));
        let shutdown_called_for_hook = Arc::clone(&shutdown_called);
        let shutdown = Shutdown::manual(move || {
            shutdown_called_for_hook.store(true, Ordering::Relaxed);
        });
        let policy =
            DaemonSessionLeasePolicy::new(Duration::from_millis(10), Duration::from_millis(5));
        let config = DaemonSessionConfig::new(policy, Duration::from_millis(10));
        let sessions = test_sessions(config, Some(shutdown));
        sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();

        let outcome = sessions.sweep_expired_at(now + Duration::from_millis(11));

        assert_eq!(outcome.expired_sessions.len(), 1);
        assert_eq!(outcome.active_sessions, 0);
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if shutdown_called.load(Ordering::Relaxed) {
                    return;
                }

                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
    }
}
