use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use synd_protocol::session::SessionId;
use tokio_util::sync::{CancellationToken, DropGuard};

use crate::shutdown::{Shutdown, ShutdownReason};

#[derive(Debug, Default)]
pub(super) struct DaemonSessionsState {
    sessions: HashMap<SessionId, SessionLeaseDeadline>,
    // Dropping the guard cancels the check for this idle period.
    pub(super) idle_shutdown_timer: Option<DropGuard>,
}

impl DaemonSessionsState {
    pub(super) fn insert(&mut self, session_id: SessionId, lease_deadline: SessionLeaseDeadline) {
        self.sessions.insert(session_id, lease_deadline);
    }

    pub(super) fn renew(
        &mut self,
        session_id: &SessionId,
        now: Instant,
        lease_deadline: SessionLeaseDeadline,
    ) -> SessionRenewal {
        let Some(deadline) = self.sessions.get_mut(session_id) else {
            return SessionRenewal::Unknown;
        };

        if deadline.is_expired(now) {
            self.sessions.remove(session_id);
            return SessionRenewal::Expired;
        }

        *deadline = lease_deadline;
        SessionRenewal::Renewed
    }

    pub(super) fn remove(&mut self, session_id: &SessionId) -> bool {
        self.sessions.remove(session_id).is_some()
    }

    pub(super) fn sweep_expired(&mut self, now: Instant) -> SessionSweep {
        let expired_sessions = self
            .sessions
            .extract_if(|_, deadline| deadline.is_expired(now))
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        SessionSweep {
            expired_sessions,
            active_sessions: self.active_session_count(),
        }
    }

    pub(super) fn active_session_count(&self) -> usize {
        self.sessions.len()
    }

    pub(super) fn idle_shutdown_pending(&self) -> bool {
        self.idle_shutdown_timer.is_some()
    }

    pub(super) fn shutdown_if_still_idle(
        &mut self,
        timer: &CancellationToken,
        shutdown: &Shutdown,
    ) -> bool {
        if !self.sessions.is_empty()
            || self.idle_shutdown_timer.as_ref().map(DropGuard::token) != Some(timer)
        {
            return false;
        }
        self.idle_shutdown_timer = None;
        shutdown.shutdown_with_reason(ShutdownReason::Idle);
        true
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

pub(super) enum SessionRenewal {
    Renewed,
    Expired,
    Unknown,
}

pub(super) struct SessionSweep {
    pub(super) expired_sessions: Vec<SessionId>,
    pub(super) active_sessions: usize,
}
