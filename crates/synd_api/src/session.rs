use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use synd_protocol::{
    CapabilitySet,
    session::{
        CloseSessionErrorResponse, CloseSessionRequest, CloseSessionResponse,
        OpenSessionErrorResponse, OpenSessionRequest, OpenSessionResponse, SessionId,
    },
};
use tokio_util::sync::CancellationToken;

use crate::shutdown::Shutdown;

pub const DEFAULT_DAEMON_IDLE_SHUTDOWN_GRACE: Duration = Duration::from_secs(30);

/// Tracks daemon sessions accepted by this API process.
#[derive(Debug, Clone)]
pub struct SessionRegistry {
    inner: Arc<SessionRegistryInner>,
}

impl Default for SessionRegistry {
    fn default() -> Self {
        Self::new(CapabilitySet::default())
    }
}

impl SessionRegistry {
    pub fn new(supported_capabilities: CapabilitySet) -> Self {
        Self::from_parts(supported_capabilities, None)
    }

    pub fn with_idle_shutdown(&self, idle_shutdown: SessionIdleShutdown) -> Self {
        Self::from_parts(self.supported_capabilities().clone(), Some(idle_shutdown))
    }

    fn from_parts(
        supported_capabilities: CapabilitySet,
        idle_shutdown: Option<SessionIdleShutdown>,
    ) -> Self {
        Self {
            inner: Arc::new(SessionRegistryInner {
                supported_capabilities,
                id_issuer: SessionIdIssuer::default(),
                idle_shutdown,
                state: Mutex::new(SessionRegistryState::default()),
            }),
        }
    }

    pub fn open(
        &self,
        request: OpenSessionRequest,
    ) -> Result<OpenSessionResponse, OpenSessionErrorResponse> {
        let context = SessionOpenContext::new(
            request.required_capabilities().clone(),
            self.supported_capabilities().clone(),
        );

        match SessionOpenDecision::from(context) {
            SessionOpenDecision::Accept { capabilities } => {
                let session_id = self.inner.id_issuer.issue();
                let effect = self.lock_state().insert(SessionRecord::new(
                    session_id.clone(),
                    request.required_capabilities().clone(),
                ));
                effect.apply(self.inner.idle_shutdown.as_ref());

                Ok(OpenSessionResponse::new(session_id, capabilities))
            }
            SessionOpenDecision::RejectMissingCapabilities {
                missing_capabilities,
            } => Err(OpenSessionErrorResponse::from_missing_capabilities(
                missing_capabilities,
            )),
        }
    }

    pub fn close(
        &self,
        request: CloseSessionRequest,
    ) -> Result<CloseSessionResponse, CloseSessionErrorResponse> {
        let session_id = request.session_id().clone();
        let context = {
            let mut state = self.lock_state();
            let removed = state.remove(&session_id, self.inner.idle_shutdown.is_some());

            SessionCloseContext::new(session_id, removed.known_session, removed.effect)
        };

        match SessionCloseDecision::from(context) {
            SessionCloseDecision::Accept { effect } => {
                effect.apply(self.inner.idle_shutdown.as_ref());
                Ok(CloseSessionResponse::new())
            }
            SessionCloseDecision::RejectUnknownSession { session_id } => {
                Err(CloseSessionErrorResponse::unknown_session(session_id))
            }
        }
    }

    fn supported_capabilities(&self) -> &CapabilitySet {
        &self.inner.supported_capabilities
    }

    fn lock_state(&self) -> MutexGuard<'_, SessionRegistryState> {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    #[cfg(test)]
    fn active_session_count(&self) -> usize {
        self.lock_state().sessions.len()
    }
}

#[derive(Debug)]
struct SessionRegistryInner {
    supported_capabilities: CapabilitySet,
    id_issuer: SessionIdIssuer,
    idle_shutdown: Option<SessionIdleShutdown>,
    state: Mutex<SessionRegistryState>,
}

#[derive(Debug, Default)]
struct SessionRegistryState {
    sessions: HashMap<SessionId, SessionRecord>,
    idle_shutdown_timer: Option<IdleShutdownTimer>,
}

impl SessionRegistryState {
    fn insert(&mut self, record: SessionRecord) -> SessionRegistryEffect {
        self.sessions.insert(record.id().clone(), record);

        self.idle_shutdown_timer
            .take()
            .map_or(SessionRegistryEffect::None, |timer| {
                SessionRegistryEffect::CancelIdleShutdown { timer }
            })
    }

    fn remove(&mut self, session_id: &SessionId, idle_shutdown_enabled: bool) -> SessionCloseFacts {
        let known_session = self.sessions.remove(session_id).is_some();
        let effect = if known_session && self.sessions.is_empty() && idle_shutdown_enabled {
            let timer = IdleShutdownTimer::new();
            self.idle_shutdown_timer = Some(timer.clone());

            SessionRegistryEffect::ScheduleIdleShutdown { timer }
        } else {
            SessionRegistryEffect::None
        };

        SessionCloseFacts {
            known_session,
            effect,
        }
    }
}

/// Facts collected from mutating session registry state for a close request.
#[derive(Debug, Clone)]
struct SessionCloseFacts {
    known_session: bool,
    effect: SessionRegistryEffect,
}

/// In-memory record for one accepted daemon session.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionRecord {
    id: SessionId,
    required_capabilities: CapabilitySet,
}

impl SessionRecord {
    fn new(id: SessionId, required_capabilities: CapabilitySet) -> Self {
        Self {
            id,
            required_capabilities,
        }
    }

    fn id(&self) -> &SessionId {
        &self.id
    }
}

/// Idle shutdown policy for a daemon session registry.
#[derive(Clone)]
pub struct SessionIdleShutdown {
    grace: Duration,
    shutdown: Shutdown,
}

impl SessionIdleShutdown {
    pub fn new(grace: Duration, shutdown: Shutdown) -> Self {
        Self { grace, shutdown }
    }

    fn schedule(&self, timer: IdleShutdownTimer) {
        let grace = self.grace;
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            tokio::select! {
                () = timer.cancelled() => {}
                () = tokio::time::sleep(grace) => {
                    tracing::info!(
                        idle_shutdown_grace_ms = grace.as_millis(),
                        "Daemon session idle grace elapsed"
                    );
                    shutdown.shutdown();
                }
            }
        });
    }
}

impl std::fmt::Debug for SessionIdleShutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionIdleShutdown")
            .field("grace", &self.grace)
            .finish_non_exhaustive()
    }
}

/// Cancellable timer token for one pending idle shutdown.
#[derive(Clone)]
struct IdleShutdownTimer {
    cancellation: CancellationToken,
}

impl IdleShutdownTimer {
    fn new() -> Self {
        Self {
            cancellation: CancellationToken::new(),
        }
    }

    fn cancel(self) {
        self.cancellation.cancel();
    }

    async fn cancelled(&self) {
        self.cancellation.cancelled().await;
    }
}

impl std::fmt::Debug for IdleShutdownTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdleShutdownTimer").finish_non_exhaustive()
    }
}

/// Side effect selected after session registry state changes.
#[derive(Debug, Clone)]
enum SessionRegistryEffect {
    None,
    CancelIdleShutdown { timer: IdleShutdownTimer },
    ScheduleIdleShutdown { timer: IdleShutdownTimer },
}

impl PartialEq for SessionRegistryEffect {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for SessionRegistryEffect {}

impl SessionRegistryEffect {
    fn apply(self, idle_shutdown: Option<&SessionIdleShutdown>) {
        match self {
            Self::None => {}
            Self::CancelIdleShutdown { timer } => timer.cancel(),
            Self::ScheduleIdleShutdown { timer } => {
                if let Some(idle_shutdown) = idle_shutdown {
                    idle_shutdown.schedule(timer);
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct SessionIdIssuer {
    next: AtomicU64,
}

impl SessionIdIssuer {
    fn issue(&self) -> SessionId {
        let id = self.next.fetch_add(1, Ordering::Relaxed);

        SessionId::new(format!("session-{id}"))
    }
}

/// Facts used to decide whether a session open request can be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionOpenContext {
    required_capabilities: CapabilitySet,
    supported_capabilities: CapabilitySet,
}

impl SessionOpenContext {
    fn new(required_capabilities: CapabilitySet, supported_capabilities: CapabilitySet) -> Self {
        Self {
            required_capabilities,
            supported_capabilities,
        }
    }
}

/// Branch selected for a session open request.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionOpenDecision {
    Accept { capabilities: CapabilitySet },
    RejectMissingCapabilities { missing_capabilities: CapabilitySet },
}

impl From<SessionOpenContext> for SessionOpenDecision {
    fn from(context: SessionOpenContext) -> Self {
        let missing_capabilities = context
            .required_capabilities
            .missing_from(&context.supported_capabilities);

        if missing_capabilities.is_empty() {
            Self::Accept {
                capabilities: context.supported_capabilities,
            }
        } else {
            Self::RejectMissingCapabilities {
                missing_capabilities,
            }
        }
    }
}

/// Facts used to decide whether a session close request can be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SessionCloseContext {
    session_id: SessionId,
    known_session: bool,
    effect: SessionRegistryEffect,
}

impl SessionCloseContext {
    fn new(session_id: SessionId, known_session: bool, effect: SessionRegistryEffect) -> Self {
        Self {
            session_id,
            known_session,
            effect,
        }
    }
}

/// Branch selected for a session close request.
#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionCloseDecision {
    Accept { effect: SessionRegistryEffect },
    RejectUnknownSession { session_id: SessionId },
}

impl From<SessionCloseContext> for SessionCloseDecision {
    fn from(context: SessionCloseContext) -> Self {
        if context.known_session {
            Self::Accept {
                effect: context.effect,
            }
        } else {
            Self::RejectUnknownSession {
                session_id: context.session_id,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use synd_protocol::{
        CapabilitySet,
        session::{CloseSessionRequest, OpenSessionRequest, SessionId},
    };

    use crate::shutdown::Shutdown;

    use super::{
        SessionCloseContext, SessionCloseDecision, SessionIdleShutdown, SessionOpenContext,
        SessionOpenDecision, SessionRegistry, SessionRegistryEffect,
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
        let registry = SessionRegistry::new(CapabilitySet::new(["timeline.read"]));

        let opened = registry
            .open(OpenSessionRequest::new(CapabilitySet::new([
                "timeline.read",
            ])))
            .unwrap();

        assert_eq!(registry.active_session_count(), 1);

        registry
            .close(CloseSessionRequest::new(opened.session_id().clone()))
            .unwrap();

        assert_eq!(registry.active_session_count(), 0);
    }

    #[test]
    fn rejects_session_when_required_capability_is_missing() {
        let registry = SessionRegistry::default();

        let error = registry
            .open(OpenSessionRequest::new(CapabilitySet::new([
                "timeline.read",
            ])))
            .unwrap_err();

        assert_eq!(error.missing_capabilities().names(), ["timeline.read"]);
        assert_eq!(registry.active_session_count(), 0);
    }

    #[test]
    fn selects_close_decision_from_known_session() {
        let cases = [
            (
                SessionCloseContext::new(
                    SessionId::new("session-1"),
                    true,
                    SessionRegistryEffect::None,
                ),
                SessionCloseDecision::Accept {
                    effect: SessionRegistryEffect::None,
                },
            ),
            (
                SessionCloseContext::new(
                    SessionId::new("session-1"),
                    false,
                    SessionRegistryEffect::None,
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

    #[tokio::test]
    async fn schedules_idle_shutdown_after_last_session_closes() {
        let shutdown_called = Arc::new(AtomicBool::new(false));
        let shutdown_called_for_hook = Arc::clone(&shutdown_called);
        let shutdown = Shutdown::manual(move || {
            shutdown_called_for_hook.store(true, Ordering::Relaxed);
        });
        let registry = SessionRegistry::default().with_idle_shutdown(SessionIdleShutdown::new(
            Duration::from_millis(10),
            shutdown,
        ));
        let opened = registry
            .open(OpenSessionRequest::new(CapabilitySet::default()))
            .unwrap();

        registry
            .close(CloseSessionRequest::new(opened.session_id().clone()))
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
        let registry = SessionRegistry::default().with_idle_shutdown(SessionIdleShutdown::new(
            Duration::from_millis(20),
            shutdown,
        ));
        let first = registry
            .open(OpenSessionRequest::new(CapabilitySet::default()))
            .unwrap();

        registry
            .close(CloseSessionRequest::new(first.session_id().clone()))
            .unwrap();
        let _second = registry
            .open(OpenSessionRequest::new(CapabilitySet::default()))
            .unwrap();
        tokio::time::sleep(Duration::from_millis(60)).await;

        assert!(!shutdown_called.load(Ordering::Relaxed));
    }
}
