use std::time::Duration;

use crate::{CapabilitySet, Result};
use synd_protocol::session::{CloseSessionRequest, SessionId};

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
    Daemon(DaemonSessionHandle),
}

impl Handle {
    pub fn inert() -> Self {
        Self {
            kind: HandleKind::Inert,
        }
    }

    pub(crate) fn daemon(client: synd_client::Client, session_id: SessionId) -> Self {
        Self {
            kind: HandleKind::Daemon(DaemonSessionHandle::new(client, session_id)),
        }
    }

    pub async fn close(self) -> Result<()> {
        match self.kind {
            HandleKind::Inert => Ok(()),
            HandleKind::Daemon(handle) => handle.close().await,
        }
    }
}

/// Client-side handle used to close an accepted daemon session.
struct DaemonSessionHandle {
    client: synd_client::Client,
    session_id: SessionId,
}

impl DaemonSessionHandle {
    fn new(client: synd_client::Client, session_id: SessionId) -> Self {
        Self { client, session_id }
    }

    async fn close(self) -> Result<()> {
        self.client
            .close_session(CloseSessionRequest::new(self.session_id))
            .await?;

        Ok(())
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
