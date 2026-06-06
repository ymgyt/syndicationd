use std::time::Duration;

use crate::{CapabilitySet, Result};
#[cfg(test)]
pub(crate) use renewal::SessionRenewalObserver;
use renewal::{SessionLeaseRenewal, SessionLeaseRenewalHandle, SessionRenewalSchedule};
use synd_protocol::session::{CloseSessionRequest, SessionId, SessionLease};

mod renewal;

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

    #[cfg(not(test))]
    pub(crate) fn daemon(
        client: synd_client::Client,
        session_id: SessionId,
        lease: SessionLease,
    ) -> Self {
        Self {
            kind: HandleKind::Daemon(DaemonSessionHandle::new(client, session_id, lease)),
        }
    }

    #[cfg(test)]
    pub(crate) fn daemon(
        client: synd_client::Client,
        session_id: SessionId,
        lease: SessionLease,
        renewal_observer: Option<SessionRenewalObserver>,
    ) -> Self {
        Self {
            kind: HandleKind::Daemon(DaemonSessionHandle::new(
                client,
                session_id,
                lease,
                renewal_observer,
            )),
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
    renewal: SessionLeaseRenewalHandle,
}

impl DaemonSessionHandle {
    #[cfg(not(test))]
    fn new(client: synd_client::Client, session_id: SessionId, lease: SessionLease) -> Self {
        let renewal = {
            let schedule = SessionRenewalSchedule::from_lease(lease);
            SessionLeaseRenewal::new(client.clone(), session_id.clone(), schedule).spawn()
        };

        Self {
            client,
            session_id,
            renewal,
        }
    }

    #[cfg(test)]
    fn new(
        client: synd_client::Client,
        session_id: SessionId,
        lease: SessionLease,
        renewal_observer: Option<SessionRenewalObserver>,
    ) -> Self {
        let renewal = {
            let schedule = SessionRenewalSchedule::from_lease(lease);
            SessionLeaseRenewal::new(
                client.clone(),
                session_id.clone(),
                schedule,
                renewal_observer,
            )
            .spawn()
        };

        Self {
            client,
            session_id,
            renewal,
        }
    }

    async fn close(self) -> Result<()> {
        self.renewal.stop().await;
        self.client
            .close_session(CloseSessionRequest::new(self.session_id))
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    acquire_timeout: Duration,
    #[cfg(test)]
    renewal_observer: Option<SessionRenewalObserver>,
}

impl Config {
    pub fn new(acquire_timeout: Duration) -> Self {
        Self {
            acquire_timeout,
            #[cfg(test)]
            renewal_observer: None,
        }
    }

    pub fn acquire_timeout(&self) -> Duration {
        self.acquire_timeout
    }

    #[cfg(test)]
    pub(crate) fn with_renewal_observer(mut self, observer: SessionRenewalObserver) -> Self {
        self.renewal_observer = Some(observer);
        self
    }

    #[cfg(test)]
    pub(crate) fn renewal_observer(&self) -> Option<SessionRenewalObserver> {
        self.renewal_observer.clone()
    }
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.acquire_timeout == other.acquire_timeout
    }
}

impl Eq for Config {}

impl Default for Config {
    fn default() -> Self {
        Self::new(Duration::from_secs(5))
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
