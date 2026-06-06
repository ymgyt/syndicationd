use synd_protocol::{
    CapabilitySet,
    session::{SessionId, SessionLease},
};

use super::{
    idle_shutdown::DaemonSessionsEffect,
    state::{SessionLeaseDeadline, SessionRenewChange, SessionRenewState, SessionSweepChange},
};

/// Inputs used to decide whether a session open request can be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionOpenContext {
    required_capabilities: CapabilitySet,
    supported_capabilities: CapabilitySet,
}

impl SessionOpenContext {
    pub(super) fn new(
        required_capabilities: CapabilitySet,
        supported_capabilities: CapabilitySet,
    ) -> Self {
        Self {
            required_capabilities,
            supported_capabilities,
        }
    }
}

/// Branch selected for a session open request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionOpenDecision {
    Accept {
        available_capabilities: CapabilitySet,
    },
    RejectMissingCapabilities {
        missing_capabilities: CapabilitySet,
    },
}

impl From<SessionOpenContext> for SessionOpenDecision {
    fn from(context: SessionOpenContext) -> Self {
        let missing_capabilities = context
            .required_capabilities
            .missing_from(&context.supported_capabilities);

        if missing_capabilities.is_empty() {
            Self::Accept {
                available_capabilities: context.supported_capabilities,
            }
        } else {
            Self::RejectMissingCapabilities {
                missing_capabilities,
            }
        }
    }
}

/// Inputs used to decide whether a session renew request can be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionRenewContext {
    session_id: SessionId,
    lease: SessionLease,
    lease_deadline: SessionLeaseDeadline,
    renew: SessionRenewChange,
}

impl SessionRenewContext {
    pub(super) fn new(
        session_id: SessionId,
        lease: SessionLease,
        lease_deadline: SessionLeaseDeadline,
        renew: SessionRenewChange,
    ) -> Self {
        Self {
            session_id,
            lease,
            lease_deadline,
            renew,
        }
    }
}

/// Branch selected for a session renew request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionRenewDecision {
    Accept {
        session_id: SessionId,
        lease: SessionLease,
        lease_deadline: SessionLeaseDeadline,
    },
    RejectExpiredSession {
        session_id: SessionId,
        effect: DaemonSessionsEffect,
    },
    RejectUnknownSession {
        session_id: SessionId,
    },
}

impl From<SessionRenewContext> for SessionRenewDecision {
    fn from(context: SessionRenewContext) -> Self {
        match context.renew.state {
            SessionRenewState::Renewed => Self::Accept {
                session_id: context.session_id,
                lease: context.lease,
                lease_deadline: context.lease_deadline,
            },
            SessionRenewState::Expired => Self::RejectExpiredSession {
                session_id: context.session_id,
                effect: context.renew.effect,
            },
            SessionRenewState::Unknown => Self::RejectUnknownSession {
                session_id: context.session_id,
            },
        }
    }
}

/// Inputs used to decide whether a session close request can be accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionCloseContext {
    session_id: SessionId,
    known_session: bool,
    effect: DaemonSessionsEffect,
}

impl SessionCloseContext {
    pub(super) fn new(
        session_id: SessionId,
        known_session: bool,
        effect: DaemonSessionsEffect,
    ) -> Self {
        Self {
            session_id,
            known_session,
            effect,
        }
    }
}

/// Branch selected for a session close request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionCloseDecision {
    Accept { effect: DaemonSessionsEffect },
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

/// Branch selected after a daemon session sweep.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SessionSweepDecision {
    NoExpiredSessions {
        active_sessions: usize,
    },
    ExpiredSessions {
        expired_sessions: Vec<SessionId>,
        active_sessions: usize,
        effect: DaemonSessionsEffect,
    },
}

impl From<SessionSweepChange> for SessionSweepDecision {
    fn from(change: SessionSweepChange) -> Self {
        if change.expired_sessions.is_empty() {
            Self::NoExpiredSessions {
                active_sessions: change.active_sessions,
            }
        } else {
            Self::ExpiredSessions {
                expired_sessions: change.expired_sessions,
                active_sessions: change.active_sessions,
                effect: change.effect,
            }
        }
    }
}
