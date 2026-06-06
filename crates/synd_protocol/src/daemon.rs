use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const STATUS_PATH: &str = "/daemon/status";

/// Response body returned by the daemon status endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonStatusResponse {
    sessions: DaemonSessionStatus,
}

impl DaemonStatusResponse {
    pub fn new(sessions: DaemonSessionStatus) -> Self {
        Self { sessions }
    }

    pub fn sessions(&self) -> &DaemonSessionStatus {
        &self.sessions
    }
}

/// Snapshot of daemon session lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonSessionStatus {
    active_sessions: usize,
    lease_duration: Duration,
    sweep_interval: Duration,
    idle_shutdown: DaemonIdleShutdownStatus,
}

impl DaemonSessionStatus {
    pub fn new(
        active_sessions: usize,
        lease_duration: Duration,
        sweep_interval: Duration,
        idle_shutdown: DaemonIdleShutdownStatus,
    ) -> Self {
        Self {
            active_sessions,
            lease_duration,
            sweep_interval,
            idle_shutdown,
        }
    }

    pub fn active_sessions(&self) -> usize {
        self.active_sessions
    }

    pub fn lease_duration(&self) -> Duration {
        self.lease_duration
    }

    pub fn sweep_interval(&self) -> Duration {
        self.sweep_interval
    }

    pub fn idle_shutdown(&self) -> &DaemonIdleShutdownStatus {
        &self.idle_shutdown
    }
}

/// Snapshot of daemon idle-shutdown state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonIdleShutdownStatus {
    enabled: bool,
    grace: Option<Duration>,
    pending: bool,
}

impl DaemonIdleShutdownStatus {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            grace: None,
            pending: false,
        }
    }

    pub fn enabled(grace: Duration, pending: bool) -> Self {
        Self {
            enabled: true,
            grace: Some(grace),
            pending,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn grace(&self) -> Option<Duration> {
        self.grace
    }

    pub fn is_pending(&self) -> bool {
        self.pending
    }
}
