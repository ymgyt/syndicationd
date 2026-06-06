use std::path::{Path, PathBuf};

use synd_protocol::daemon::DaemonSessionStatus;

use crate::placement::RuntimePlacement;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Running,
    NotRunning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    state: State,
    placement: RuntimePlacementSummary,
    sessions: Option<DaemonSessionStatus>,
}

impl Status {
    pub(crate) fn new(state: State, placement: RuntimePlacementSummary) -> Self {
        Self {
            state,
            placement,
            sessions: None,
        }
    }

    pub(crate) fn running(
        placement: RuntimePlacementSummary,
        sessions: DaemonSessionStatus,
    ) -> Self {
        Self {
            state: State::Running,
            placement,
            sessions: Some(sessions),
        }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn placement(&self) -> &RuntimePlacementSummary {
        &self.placement
    }

    pub fn sessions(&self) -> Option<&DaemonSessionStatus> {
        self.sessions.as_ref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlacementSummary {
    runtime_root: PathBuf,
    runtime_instance_id: String,
    database: PathBuf,
    endpoint: PathBuf,
    startup_lock: PathBuf,
}

impl RuntimePlacementSummary {
    pub(crate) fn from_placement(placement: &RuntimePlacement) -> Self {
        Self {
            runtime_root: placement.root().path().to_path_buf(),
            runtime_instance_id: placement.instance().id().to_string(),
            database: placement.instance().canonical_database_path().to_path_buf(),
            endpoint: placement.endpoint().path().to_path_buf(),
            startup_lock: placement.startup_lock_path().path().to_path_buf(),
        }
    }

    pub fn runtime_root(&self) -> &Path {
        &self.runtime_root
    }

    pub fn runtime_instance_id(&self) -> &str {
        &self.runtime_instance_id
    }

    pub fn database(&self) -> &Path {
        &self.database
    }

    pub fn endpoint(&self) -> &Path {
        &self.endpoint
    }

    pub fn startup_lock(&self) -> &Path {
        &self.startup_lock
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShutdownResult {
    status: Status,
}

impl ShutdownResult {
    pub fn new(status: Status) -> Self {
        Self { status }
    }

    pub fn status(&self) -> &Status {
        &self.status
    }
}
