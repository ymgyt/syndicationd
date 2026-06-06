use std::{
    collections::HashMap,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};

use synd_protocol::{CapabilitySet, session::SessionId};

use super::idle_shutdown::{DaemonSessionsEffect, IdleShutdownTimer};

#[derive(Debug, Default)]
pub(super) struct DaemonSessionsState {
    sessions: HashMap<SessionId, DaemonSession>,
    idle_shutdown_timer: Option<IdleShutdownTimer>,
}

impl DaemonSessionsState {
    pub(super) fn insert(&mut self, session: DaemonSession) -> DaemonSessionsEffect {
        self.sessions.insert(session.id().clone(), session);

        self.idle_shutdown_timer
            .take()
            .map_or(DaemonSessionsEffect::None, |timer| {
                DaemonSessionsEffect::CancelIdleShutdown { timer }
            })
    }

    pub(super) fn renew(
        &mut self,
        session_id: &SessionId,
        now: Instant,
        lease_deadline: SessionLeaseDeadline,
        idle_shutdown_enabled: bool,
    ) -> SessionRenewChange {
        let Some(session) = self.sessions.get_mut(session_id) else {
            return SessionRenewChange::unknown();
        };

        if session.is_expired(now) {
            self.sessions.remove(session_id);
            let effect = if self.sessions.is_empty() && idle_shutdown_enabled {
                self.schedule_idle_shutdown()
            } else {
                DaemonSessionsEffect::None
            };

            return SessionRenewChange::expired(effect);
        }

        session.renew(lease_deadline);
        SessionRenewChange::renewed()
    }

    pub(super) fn remove(
        &mut self,
        session_id: &SessionId,
        idle_shutdown_enabled: bool,
    ) -> SessionCloseChange {
        let known_session = self.sessions.remove(session_id).is_some();
        let effect = if known_session && self.sessions.is_empty() && idle_shutdown_enabled {
            self.schedule_idle_shutdown()
        } else {
            DaemonSessionsEffect::None
        };

        SessionCloseChange {
            known_session,
            effect,
        }
    }

    pub(super) fn sweep_expired(
        &mut self,
        now: Instant,
        idle_shutdown_enabled: bool,
    ) -> SessionSweepChange {
        let expired_sessions = self
            .sessions
            .values()
            .filter(|session| session.is_expired(now))
            .map(|session| session.id().clone())
            .collect::<Vec<_>>();

        for session_id in &expired_sessions {
            self.sessions.remove(session_id);
        }

        let active_sessions = self.active_session_count();
        let effect =
            if !expired_sessions.is_empty() && active_sessions == 0 && idle_shutdown_enabled {
                self.schedule_idle_shutdown()
            } else {
                DaemonSessionsEffect::None
            };

        SessionSweepChange::new(expired_sessions, active_sessions, effect)
    }

    pub(super) fn active_session_count(&self) -> usize {
        self.sessions.len()
    }

    pub(super) fn idle_shutdown_pending(&self) -> bool {
        self.idle_shutdown_timer.is_some()
    }

    fn schedule_idle_shutdown(&mut self) -> DaemonSessionsEffect {
        let timer = IdleShutdownTimer::new();
        self.idle_shutdown_timer = Some(timer.clone());

        DaemonSessionsEffect::ScheduleIdleShutdown { timer }
    }
}

/// State change produced by closing a daemon session.
#[derive(Debug, Clone)]
pub(super) struct SessionCloseChange {
    pub(super) known_session: bool,
    pub(super) effect: DaemonSessionsEffect,
}

/// In-memory state for one accepted daemon session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DaemonSession {
    id: SessionId,
    required_capabilities: CapabilitySet,
    lease_deadline: SessionLeaseDeadline,
}

impl DaemonSession {
    pub(super) fn new(
        id: SessionId,
        required_capabilities: CapabilitySet,
        lease_deadline: SessionLeaseDeadline,
    ) -> Self {
        Self {
            id,
            required_capabilities,
            lease_deadline,
        }
    }

    fn id(&self) -> &SessionId {
        &self.id
    }

    fn is_expired(&self, now: Instant) -> bool {
        self.lease_deadline.is_expired(now)
    }

    fn renew(&mut self, lease_deadline: SessionLeaseDeadline) {
        self.lease_deadline = lease_deadline;
    }
}

/// Absolute expiration deadline for one daemon session lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SessionLeaseDeadline {
    expires_at: Instant,
}

impl SessionLeaseDeadline {
    pub(super) fn new(expires_at: Instant) -> Self {
        Self { expires_at }
    }

    fn is_expired(self, now: Instant) -> bool {
        now >= self.expires_at
    }

    pub(super) fn remaining_from(self, now: Instant) -> Duration {
        self.expires_at.saturating_duration_since(now)
    }
}

/// State change produced by renewing a daemon session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionRenewChange {
    pub(super) state: SessionRenewState,
    pub(super) effect: DaemonSessionsEffect,
}

impl SessionRenewChange {
    pub(super) fn renewed() -> Self {
        Self {
            state: SessionRenewState::Renewed,
            effect: DaemonSessionsEffect::None,
        }
    }

    pub(super) fn expired(effect: DaemonSessionsEffect) -> Self {
        Self {
            state: SessionRenewState::Expired,
            effect,
        }
    }

    pub(super) fn unknown() -> Self {
        Self {
            state: SessionRenewState::Unknown,
            effect: DaemonSessionsEffect::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SessionRenewState {
    Renewed,
    Expired,
    Unknown,
}

/// State change produced by one daemon session sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionSweepChange {
    pub(super) expired_sessions: Vec<SessionId>,
    pub(super) active_sessions: usize,
    pub(super) effect: DaemonSessionsEffect,
}

impl SessionSweepChange {
    pub(super) fn new(
        expired_sessions: Vec<SessionId>,
        active_sessions: usize,
        effect: DaemonSessionsEffect,
    ) -> Self {
        Self {
            expired_sessions,
            active_sessions,
            effect,
        }
    }
}

/// Result of one daemon session sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SessionSweepOutcome {
    expired_session_count: usize,
    active_sessions: usize,
}

impl SessionSweepOutcome {
    pub(super) fn new(expired_session_count: usize, active_sessions: usize) -> Self {
        Self {
            expired_session_count,
            active_sessions,
        }
    }

    pub(super) fn expired_session_count(self) -> usize {
        self.expired_session_count
    }

    pub(super) fn active_sessions(self) -> usize {
        self.active_sessions
    }
}

#[derive(Debug, Default)]
pub(super) struct SessionIdIssuer {
    next: AtomicU64,
}

impl SessionIdIssuer {
    pub(super) fn issue(&self) -> SessionId {
        let id = self.next.fetch_add(1, Ordering::Relaxed);

        SessionId::new(format!("session-{id}"))
    }
}
