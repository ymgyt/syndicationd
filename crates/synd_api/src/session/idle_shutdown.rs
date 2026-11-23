use std::{sync::Arc, time::Duration};

use tokio_util::sync::{CancellationToken, DropGuard};
use tracing::info;

use super::DaemonSessions;
use crate::shutdown::Shutdown;

/// Idle shutdown policy for daemon sessions.
pub(super) struct SessionIdleShutdown {
    grace: Duration,
    shutdown: Shutdown,
}

impl SessionIdleShutdown {
    pub(super) fn new(grace: Duration, shutdown: Shutdown) -> Self {
        Self { grace, shutdown }
    }

    pub(super) fn grace(&self) -> Duration {
        self.grace
    }

    pub(super) fn schedule_check(&self, sessions: &DaemonSessions) -> DropGuard {
        let sessions = Arc::downgrade(&sessions.inner);
        let timer = CancellationToken::new();
        let guard = timer.clone().drop_guard();
        let grace = self.grace;
        let wait = tokio::time::sleep(grace);
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            tokio::select! {
                () = timer.cancelled() => return,
                () = wait => {}
            }
            let Some(inner) = sessions.upgrade() else {
                return;
            };
            let sessions = DaemonSessions { inner };
            if sessions
                .lock_state()
                .shutdown_if_still_idle(&timer, &shutdown)
            {
                info!(
                    idle_shutdown_grace_ms = grace.as_millis(),
                    "Daemon session idle grace elapsed"
                );
            }
        });
        guard
    }
}

impl std::fmt::Debug for SessionIdleShutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionIdleShutdown")
            .field("grace", &self.grace)
            .finish_non_exhaustive()
    }
}
