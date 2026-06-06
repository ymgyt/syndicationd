use std::{
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant},
};

use synd_protocol::{
    CapabilitySet,
    session::{
        CloseSessionErrorResponse, CloseSessionRequest, CloseSessionResponse,
        OpenSessionErrorResponse, OpenSessionRequest, OpenSessionResponse,
        RenewSessionErrorResponse, RenewSessionRequest, RenewSessionResponse, SessionLease,
    },
};
use tracing::debug;

mod decision;
mod idle_shutdown;
mod state;
mod sweeper;

pub use idle_shutdown::SessionIdleShutdown;
pub(crate) use sweeper::DaemonSessionSweeper;

use decision::{
    SessionCloseContext, SessionCloseDecision, SessionOpenContext, SessionOpenDecision,
    SessionRenewContext, SessionRenewDecision, SessionSweepDecision,
};
use state::{
    DaemonSession, DaemonSessionsState, SessionIdIssuer, SessionLeaseDeadline, SessionSweepOutcome,
};

pub const DEFAULT_DAEMON_IDLE_SHUTDOWN_GRACE: Duration = Duration::from_secs(30);
pub const DEFAULT_DAEMON_SESSION_LEASE_DURATION: Duration = Duration::from_secs(30);
pub const DEFAULT_DAEMON_SESSION_RENEWAL_INTERVAL: Duration = Duration::from_secs(10);
pub const DEFAULT_DAEMON_SESSION_SWEEP_INTERVAL: Duration = Duration::from_secs(5);

/// Owns daemon sessions accepted by this API process.
#[derive(Debug, Clone)]
pub struct DaemonSessions {
    inner: Arc<DaemonSessionsInner>,
}

impl Default for DaemonSessions {
    fn default() -> Self {
        Self::new(CapabilitySet::default())
    }
}

impl DaemonSessions {
    pub fn new(supported_capabilities: CapabilitySet) -> Self {
        Self::from_parts(
            supported_capabilities,
            DaemonSessionLeasePolicy::default(),
            None,
        )
    }

    #[must_use]
    pub fn with_idle_shutdown(&self, idle_shutdown: SessionIdleShutdown) -> Self {
        Self::from_parts(
            self.supported_capabilities().clone(),
            self.inner.lease_policy,
            Some(idle_shutdown),
        )
    }

    #[must_use]
    pub fn with_lease_policy(&self, lease_policy: DaemonSessionLeasePolicy) -> Self {
        Self::from_parts(
            self.supported_capabilities().clone(),
            lease_policy,
            self.inner.idle_shutdown.clone(),
        )
    }

    fn from_parts(
        supported_capabilities: CapabilitySet,
        lease_policy: DaemonSessionLeasePolicy,
        idle_shutdown: Option<SessionIdleShutdown>,
    ) -> Self {
        Self {
            inner: Arc::new(DaemonSessionsInner {
                supported_capabilities,
                lease_policy,
                id_issuer: SessionIdIssuer::default(),
                idle_shutdown,
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
        let context = SessionOpenContext::new(
            request.required_capabilities().clone(),
            self.supported_capabilities().clone(),
        );

        match SessionOpenDecision::from(context) {
            SessionOpenDecision::Accept { capabilities } => {
                let session_id = self.inner.id_issuer.issue();
                let lease = self.inner.lease_policy.lease();
                let lease_deadline = self.inner.lease_policy.deadline_from(now);
                let (effect, active_sessions) = {
                    let mut state = self.lock_state();
                    let effect = state.insert(DaemonSession::new(
                        session_id.clone(),
                        request.required_capabilities().clone(),
                        lease_deadline,
                    ));

                    (effect, state.active_session_count())
                };
                effect.apply(self.inner.idle_shutdown.as_ref());
                debug!(
                    %session_id,
                    active_sessions,
                    lease_duration_ms = lease.duration().as_millis(),
                    "Opened daemon session"
                );

                Ok(OpenSessionResponse::with_lease(
                    session_id,
                    capabilities,
                    lease,
                ))
            }
            SessionOpenDecision::RejectMissingCapabilities {
                missing_capabilities,
            } => {
                debug!(
                    missing_capabilities = ?missing_capabilities,
                    "Rejected daemon session open"
                );
                Err(OpenSessionErrorResponse::from_missing_capabilities(
                    missing_capabilities,
                ))
            }
        }
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
        let lease = self.inner.lease_policy.lease();
        let lease_deadline = self.inner.lease_policy.deadline_from(now);
        let context = {
            let mut state = self.lock_state();
            let renew = state.renew(
                &session_id,
                now,
                lease_deadline,
                self.inner.idle_shutdown.is_some(),
            );

            SessionRenewContext::new(session_id, lease, lease_deadline, renew)
        };

        match SessionRenewDecision::from(context) {
            SessionRenewDecision::Accept {
                session_id,
                lease,
                lease_deadline,
            } => {
                debug!(
                    %session_id,
                    lease_expires_in_ms = lease_deadline.remaining_from(now).as_millis(),
                    "Renewed daemon session"
                );
                Ok(RenewSessionResponse::new(session_id, lease))
            }
            SessionRenewDecision::RejectUnknownSession { session_id } => {
                debug!(%session_id, "Rejected daemon session renew");
                Err(RenewSessionErrorResponse::unknown_session(session_id))
            }
            SessionRenewDecision::RejectExpiredSession { session_id, effect } => {
                effect.apply(self.inner.idle_shutdown.as_ref());
                debug!(%session_id, "Expired daemon session during renew");
                Err(RenewSessionErrorResponse::unknown_session(session_id))
            }
        }
    }

    pub fn close(
        &self,
        request: &CloseSessionRequest,
    ) -> Result<CloseSessionResponse, CloseSessionErrorResponse> {
        let session_id = request.session_id().clone();
        let context = {
            let mut state = self.lock_state();
            let removed = state.remove(&session_id, self.inner.idle_shutdown.is_some());

            SessionCloseContext::new(session_id.clone(), removed.known_session, removed.effect)
        };

        match SessionCloseDecision::from(context) {
            SessionCloseDecision::Accept { effect } => {
                effect.apply(self.inner.idle_shutdown.as_ref());
                debug!(%session_id, "Closed daemon session");
                Ok(CloseSessionResponse::new())
            }
            SessionCloseDecision::RejectUnknownSession { session_id } => {
                debug!(%session_id, "Rejected daemon session close");
                Err(CloseSessionErrorResponse::unknown_session(session_id))
            }
        }
    }

    fn sweep_interval(&self) -> Duration {
        self.inner.lease_policy.sweep_interval()
    }

    fn sweep_expired(&self) -> SessionSweepOutcome {
        self.sweep_expired_at(Instant::now())
    }

    fn sweep_expired_at(&self, now: Instant) -> SessionSweepOutcome {
        let facts = {
            let mut state = self.lock_state();
            state.sweep_expired(now, self.inner.idle_shutdown.is_some())
        };

        match SessionSweepDecision::from(facts) {
            SessionSweepDecision::NoExpiredSessions { active_sessions } => {
                SessionSweepOutcome::new(0, active_sessions)
            }
            SessionSweepDecision::ExpiredSessions {
                expired_sessions,
                active_sessions,
                effect,
            } => {
                let expired_session_count = expired_sessions.len();
                effect.apply(self.inner.idle_shutdown.as_ref());
                debug!(
                    expired_sessions = ?expired_sessions,
                    active_sessions,
                    "Expired daemon sessions during sweep"
                );

                SessionSweepOutcome::new(expired_session_count, active_sessions)
            }
        }
    }

    fn supported_capabilities(&self) -> &CapabilitySet {
        &self.inner.supported_capabilities
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

/// Lease timing policy for daemon sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DaemonSessionLeasePolicy {
    lease_duration: Duration,
    renewal_interval: Duration,
    sweep_interval: Duration,
}

impl DaemonSessionLeasePolicy {
    pub fn new(
        lease_duration: Duration,
        renewal_interval: Duration,
        sweep_interval: Duration,
    ) -> Self {
        Self {
            lease_duration,
            renewal_interval,
            sweep_interval,
        }
    }

    fn lease(self) -> SessionLease {
        SessionLease::new(self.lease_duration)
    }

    fn deadline_from(self, now: Instant) -> SessionLeaseDeadline {
        SessionLeaseDeadline::new(now + self.lease_duration)
    }

    pub fn lease_duration(self) -> Duration {
        self.lease_duration
    }

    pub fn renewal_interval(self) -> Duration {
        self.renewal_interval
    }

    pub fn sweep_interval(self) -> Duration {
        self.sweep_interval
    }
}

impl Default for DaemonSessionLeasePolicy {
    fn default() -> Self {
        Self::new(
            DEFAULT_DAEMON_SESSION_LEASE_DURATION,
            DEFAULT_DAEMON_SESSION_RENEWAL_INTERVAL,
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
        CapabilitySet,
        session::{CloseSessionRequest, OpenSessionRequest, RenewSessionRequest, SessionId},
    };

    use crate::shutdown::Shutdown;

    use super::{
        DaemonSessionLeasePolicy, DaemonSessions, SessionIdleShutdown,
        decision::{
            SessionCloseContext, SessionCloseDecision, SessionOpenContext, SessionOpenDecision,
            SessionRenewContext, SessionRenewDecision, SessionSweepDecision,
        },
        idle_shutdown::DaemonSessionsEffect,
        state::{SessionLeaseDeadline, SessionRenewChange, SessionSweepChange},
    };

    #[test]
    fn selects_open_decision_from_capabilities() {
        let cases = [
            (
                SessionOpenContext::new(
                    CapabilitySet::new(["timeline.read"]),
                    CapabilitySet::new(["timeline.read", "subscription.write"]),
                ),
                SessionOpenDecision::Accept {
                    capabilities: CapabilitySet::new(["timeline.read", "subscription.write"]),
                },
            ),
            (
                SessionOpenContext::new(
                    CapabilitySet::new(["timeline.read", "subscription.write"]),
                    CapabilitySet::new(["timeline.read"]),
                ),
                SessionOpenDecision::RejectMissingCapabilities {
                    missing_capabilities: CapabilitySet::new(["subscription.write"]),
                },
            ),
        ];

        for (context, expected) in cases {
            assert_eq!(SessionOpenDecision::from(context), expected);
        }
    }

    #[test]
    fn opens_and_closes_session() {
        let sessions = DaemonSessions::new(CapabilitySet::new(["timeline.read"]));

        let opened = sessions
            .open(&OpenSessionRequest::new(CapabilitySet::new([
                "timeline.read",
            ])))
            .unwrap();

        assert_eq!(sessions.active_session_count(), 1);

        sessions
            .close(&CloseSessionRequest::new(opened.session_id().clone()))
            .unwrap();

        assert_eq!(sessions.active_session_count(), 0);
    }

    #[test]
    fn opens_session_with_lease_policy() {
        let now = Instant::now();
        let policy = DaemonSessionLeasePolicy::new(
            Duration::from_secs(12),
            Duration::from_secs(4),
            Duration::from_secs(2),
        );
        let sessions = DaemonSessions::default().with_lease_policy(policy);

        let opened = sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();

        assert_eq!(opened.lease().duration(), policy.lease_duration());
        assert_eq!(sessions.active_session_count(), 1);
    }

    #[test]
    fn rejects_session_when_required_capability_is_missing() {
        let sessions = DaemonSessions::default();

        let error = sessions
            .open(&OpenSessionRequest::new(CapabilitySet::new([
                "timeline.read",
            ])))
            .unwrap_err();

        assert_eq!(error.missing_capabilities().names(), ["timeline.read"]);
        assert_eq!(sessions.active_session_count(), 0);
    }

    #[test]
    fn renews_session_before_lease_deadline() {
        let now = Instant::now();
        let policy = DaemonSessionLeasePolicy::new(
            Duration::from_secs(30),
            Duration::from_secs(10),
            Duration::from_secs(5),
        );
        let sessions = DaemonSessions::default().with_lease_policy(policy);
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
        let policy = DaemonSessionLeasePolicy::new(
            Duration::from_secs(30),
            Duration::from_secs(10),
            Duration::from_secs(5),
        );
        let sessions = DaemonSessions::default().with_lease_policy(policy);
        let opened = sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();

        let error = sessions
            .renew_at(
                &RenewSessionRequest::new(opened.session_id().clone()),
                now + Duration::from_secs(31),
            )
            .unwrap_err();

        assert_eq!(error.session_id(), opened.session_id());
        assert_eq!(sessions.active_session_count(), 0);
    }

    #[test]
    fn rejects_unknown_session_during_renew() {
        let sessions = DaemonSessions::default();
        let session_id = SessionId::new("session-unknown");

        let error = sessions
            .renew(&RenewSessionRequest::new(session_id.clone()))
            .unwrap_err();

        assert_eq!(error.session_id(), &session_id);
    }

    #[test]
    fn selects_renew_decision_from_change() {
        let now = Instant::now();
        let lease = synd_protocol::session::SessionLease::new(Duration::from_secs(30));
        let lease_deadline = SessionLeaseDeadline::new(now + Duration::from_secs(30));
        let cases = [
            (
                SessionRenewContext::new(
                    SessionId::new("session-1"),
                    lease,
                    lease_deadline,
                    SessionRenewChange::renewed(),
                ),
                SessionRenewDecision::Accept {
                    session_id: SessionId::new("session-1"),
                    lease,
                    lease_deadline,
                },
            ),
            (
                SessionRenewContext::new(
                    SessionId::new("session-1"),
                    lease,
                    lease_deadline,
                    SessionRenewChange::expired(DaemonSessionsEffect::None),
                ),
                SessionRenewDecision::RejectExpiredSession {
                    session_id: SessionId::new("session-1"),
                    effect: DaemonSessionsEffect::None,
                },
            ),
            (
                SessionRenewContext::new(
                    SessionId::new("session-1"),
                    lease,
                    lease_deadline,
                    SessionRenewChange::unknown(),
                ),
                SessionRenewDecision::RejectUnknownSession {
                    session_id: SessionId::new("session-1"),
                },
            ),
        ];

        for (context, expected) in cases {
            assert_eq!(SessionRenewDecision::from(context), expected);
        }
    }

    #[test]
    fn selects_close_decision_from_known_session() {
        let cases = [
            (
                SessionCloseContext::new(
                    SessionId::new("session-1"),
                    true,
                    DaemonSessionsEffect::None,
                ),
                SessionCloseDecision::Accept {
                    effect: DaemonSessionsEffect::None,
                },
            ),
            (
                SessionCloseContext::new(
                    SessionId::new("session-1"),
                    false,
                    DaemonSessionsEffect::None,
                ),
                SessionCloseDecision::RejectUnknownSession {
                    session_id: SessionId::new("session-1"),
                },
            ),
        ];

        for (context, expected) in cases {
            assert_eq!(SessionCloseDecision::from(context), expected);
        }
    }

    #[test]
    fn sweeps_expired_sessions() {
        let now = Instant::now();
        let policy = DaemonSessionLeasePolicy::new(
            Duration::from_secs(30),
            Duration::from_secs(10),
            Duration::from_secs(5),
        );
        let sessions = DaemonSessions::default().with_lease_policy(policy);
        sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();
        sessions
            .open_at(
                &OpenSessionRequest::new(CapabilitySet::default()),
                now + Duration::from_secs(10),
            )
            .unwrap();

        let outcome = sessions.sweep_expired_at(now + Duration::from_secs(31));

        assert_eq!(outcome.expired_session_count(), 1);
        assert_eq!(outcome.active_sessions(), 1);
        assert_eq!(sessions.active_session_count(), 1);
    }

    #[test]
    fn selects_sweep_decision_from_change() {
        let cases = [
            (
                SessionSweepChange::new(vec![], 2, DaemonSessionsEffect::None),
                SessionSweepDecision::NoExpiredSessions { active_sessions: 2 },
            ),
            (
                SessionSweepChange::new(
                    vec![SessionId::new("session-1")],
                    0,
                    DaemonSessionsEffect::None,
                ),
                SessionSweepDecision::ExpiredSessions {
                    expired_sessions: vec![SessionId::new("session-1")],
                    active_sessions: 0,
                    effect: DaemonSessionsEffect::None,
                },
            ),
        ];

        for (facts, expected) in cases {
            assert_eq!(SessionSweepDecision::from(facts), expected);
        }
    }

    #[tokio::test]
    async fn schedules_idle_shutdown_after_last_session_closes() {
        let shutdown_called = Arc::new(AtomicBool::new(false));
        let shutdown_called_for_hook = Arc::clone(&shutdown_called);
        let shutdown = Shutdown::manual(move || {
            shutdown_called_for_hook.store(true, Ordering::Relaxed);
        });
        let sessions = DaemonSessions::default().with_idle_shutdown(SessionIdleShutdown::new(
            Duration::from_millis(10),
            shutdown,
        ));
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
        let sessions = DaemonSessions::default().with_idle_shutdown(SessionIdleShutdown::new(
            Duration::from_millis(20),
            shutdown,
        ));
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
        let policy = DaemonSessionLeasePolicy::new(
            Duration::from_millis(10),
            Duration::from_millis(5),
            Duration::from_millis(5),
        );
        let sessions = DaemonSessions::default()
            .with_lease_policy(policy)
            .with_idle_shutdown(SessionIdleShutdown::new(
                Duration::from_millis(10),
                shutdown,
            ));
        sessions
            .open_at(&OpenSessionRequest::new(CapabilitySet::default()), now)
            .unwrap();

        let outcome = sessions.sweep_expired_at(now + Duration::from_millis(11));

        assert_eq!(outcome.expired_session_count(), 1);
        assert_eq!(outcome.active_sessions(), 0);
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
