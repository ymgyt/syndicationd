use std::time::Duration;

use crate::{CapabilitySet, Result, loopback::LoopbackApiHandle};

pub struct Session {
    client: synd_client::Client,
    capabilities: CapabilitySet,
    handle: Handle,
}

impl Session {
    pub fn new(client: synd_client::Client, capabilities: CapabilitySet, handle: Handle) -> Self {
        Self {
            client,
            capabilities,
            handle,
        }
    }

    pub fn client(&self) -> &synd_client::Client {
        &self.client
    }

    pub fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }

    pub async fn close(self) -> Result<()> {
        self.handle.close().await
    }
}

pub struct Handle {
    kind: HandleKind,
}

enum HandleKind {
    Inert,
    Loopback(LoopbackApiHandle),
}

impl Handle {
    pub fn inert() -> Self {
        Self {
            kind: HandleKind::Inert,
        }
    }

    pub async fn close(self) -> Result<()> {
        match self.kind {
            HandleKind::Inert => Ok(()),
            HandleKind::Loopback(handle) => handle.shutdown().await,
        }
    }
}

impl From<LoopbackApiHandle> for Handle {
    fn from(handle: LoopbackApiHandle) -> Self {
        Self {
            kind: HandleKind::Loopback(handle),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    acquire_timeout: Duration,
}

impl Config {
    pub fn new(acquire_timeout: Duration) -> Self {
        Self { acquire_timeout }
    }

    pub fn acquire_timeout(&self) -> Duration {
        self.acquire_timeout
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Requirements {
    capabilities: CapabilitySet,
}

impl Requirements {
    pub fn new(capabilities: CapabilitySet) -> Self {
        Self { capabilities }
    }

    pub fn capabilities(&self) -> &CapabilitySet {
        &self.capabilities
    }
}
