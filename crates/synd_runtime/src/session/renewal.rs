#[cfg(test)]
use std::fmt;
use std::time::Duration;

use synd_protocol::session::{RenewSessionRequest, SessionId, SessionLease};
#[cfg(test)]
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{debug, warn};

const MIN_RENEWAL_INTERVAL: Duration = Duration::from_millis(100);

/// Client-side controller that keeps a daemon session lease alive.
pub(super) struct SessionLeaseRenewal {
    client: synd_client::Client,
    session_id: SessionId,
    schedule: SessionRenewalSchedule,
    #[cfg(test)]
    observer: Option<SessionRenewalObserver>,
}

impl SessionLeaseRenewal {
    #[cfg(not(test))]
    pub(super) fn new(
        client: synd_client::Client,
        session_id: SessionId,
        schedule: SessionRenewalSchedule,
    ) -> Self {
        Self {
            client,
            session_id,
            schedule,
        }
    }

    #[cfg(test)]
    pub(super) fn new(
        client: synd_client::Client,
        session_id: SessionId,
        schedule: SessionRenewalSchedule,
        observer: Option<SessionRenewalObserver>,
    ) -> Self {
        Self {
            client,
            session_id,
            schedule,
            observer,
        }
    }

    pub(super) fn spawn(self) -> SessionLeaseRenewalHandle {
        let session_id = self.session_id.clone();
        debug!(
            %session_id,
            renew_after_ms = self.schedule.renew_after().as_millis(),
            "Started daemon session lease renewal"
        );

        SessionLeaseRenewalHandle {
            session_id,
            task: tokio::spawn(self.run()),
        }
    }

    async fn run(self) {
        let client = self.client;
        let session_id = self.session_id;
        let mut schedule = self.schedule;
        #[cfg(test)]
        let observer = self.observer;

        loop {
            tokio::time::sleep(schedule.renew_after()).await;

            match client
                .renew_session(RenewSessionRequest::new(session_id.clone()))
                .await
            {
                Ok(response) => {
                    schedule = SessionRenewalSchedule::from_lease(response.lease());
                    #[cfg(test)]
                    if let Some(observer) = &observer {
                        observer.session_renewed(&session_id);
                    }
                    debug!(
                        %session_id,
                        lease_duration_ms = response.lease().duration().as_millis(),
                        next_renew_after_ms = schedule.renew_after().as_millis(),
                        "Renewed daemon session lease"
                    );
                }
                Err(error) => {
                    warn!(
                        %session_id,
                        error = ?error,
                        "Stopped daemon session lease renewal after renew failure"
                    );
                    break;
                }
            }
        }
    }
}

/// Test-only synchronization point for lease renewal integration tests.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct SessionRenewalObserver {
    renewed: mpsc::UnboundedSender<SessionId>,
}

#[cfg(test)]
impl SessionRenewalObserver {
    pub(crate) fn new(renewed: mpsc::UnboundedSender<SessionId>) -> Self {
        Self { renewed }
    }

    fn session_renewed(&self, session_id: &SessionId) {
        let _ = self.renewed.send(session_id.clone());
    }
}

#[cfg(test)]
impl fmt::Debug for SessionRenewalObserver {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionRenewalObserver")
            .finish_non_exhaustive()
    }
}

/// Handle used to stop client-side daemon session lease renewal.
pub(super) struct SessionLeaseRenewalHandle {
    session_id: SessionId,
    task: JoinHandle<()>,
}

impl SessionLeaseRenewalHandle {
    pub(super) async fn stop(mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
        debug!(session_id = %self.session_id, "Stopped daemon session lease renewal");
    }
}

impl Drop for SessionLeaseRenewalHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Renewal cadence derived from a daemon-granted session lease.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct SessionRenewalSchedule {
    renew_after: Duration,
}

impl SessionRenewalSchedule {
    pub(super) fn from_lease(lease: SessionLease) -> Self {
        let renew_after = lease
            .duration()
            .checked_div(3)
            .unwrap_or(MIN_RENEWAL_INTERVAL)
            .max(MIN_RENEWAL_INTERVAL);

        Self { renew_after }
    }

    fn renew_after(self) -> Duration {
        self.renew_after
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use synd_protocol::session::SessionLease;

    use super::{MIN_RENEWAL_INTERVAL, SessionRenewalSchedule};

    mod schedule {
        use super::*;

        #[test]
        fn derives_from_lease() {
            let schedule =
                SessionRenewalSchedule::from_lease(SessionLease::new(Duration::from_secs(30)));

            assert_eq!(schedule.renew_after(), Duration::from_secs(10));
        }

        #[test]
        fn has_minimum_interval() {
            let schedule = SessionRenewalSchedule::from_lease(SessionLease::new(Duration::ZERO));

            assert_eq!(schedule.renew_after(), MIN_RENEWAL_INTERVAL);
        }
    }
}
