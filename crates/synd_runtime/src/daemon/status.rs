use std::path::{Path, PathBuf};

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
}

impl Status {
    pub(crate) fn new(state: State, placement: RuntimePlacementSummary) -> Self {
        Self { state, placement }
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn placement(&self) -> &RuntimePlacementSummary {
        &self.placement
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePlacementSummary {
    runtime_instance_id: String,
    database: PathBuf,
    endpoint: PathBuf,
}

impl RuntimePlacementSummary {
    pub(crate) fn from_placement(placement: &RuntimePlacement) -> Self {
        Self {
            runtime_instance_id: placement.instance().id().to_string(),
            database: placement.instance().canonical_database_path().to_path_buf(),
            endpoint: placement.endpoint().path().to_path_buf(),
        }
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
