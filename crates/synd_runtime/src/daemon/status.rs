#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Running,
    NotRunning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    state: State,
}

impl Status {
    pub fn new(state: State) -> Self {
        Self { state }
    }

    pub fn state(&self) -> State {
        self.state
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
