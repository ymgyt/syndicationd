use std::time::Duration;

use tokio_util::sync::CancellationToken;
use tracing::info;

use crate::shutdown::Shutdown;

/// Idle shutdown policy for daemon sessions.
#[derive(Clone)]
pub struct SessionIdleShutdown {
    grace: Duration,
    shutdown: Shutdown,
}

impl SessionIdleShutdown {
    pub fn new(grace: Duration, shutdown: Shutdown) -> Self {
        Self { grace, shutdown }
    }

    pub fn grace(&self) -> Duration {
        self.grace
    }

    fn schedule(&self, timer: IdleShutdownTimer) {
        let grace = self.grace;
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            tokio::select! {
                () = timer.cancelled() => {}
                () = tokio::time::sleep(grace) => {
                    info!(
                        idle_shutdown_grace_ms = grace.as_millis(),
                        "Daemon session idle grace elapsed"
                    );
                    shutdown.shutdown();
                }
            }
        });
    }
}

impl std::fmt::Debug for SessionIdleShutdown {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionIdleShutdown")
            .field("grace", &self.grace)
            .finish_non_exhaustive()
    }
}

/// Cancellable timer token for one pending idle shutdown.
#[derive(Clone)]
pub(super) struct IdleShutdownTimer {
    cancellation: CancellationToken,
}

impl IdleShutdownTimer {
    pub(super) fn new() -> Self {
        Self {
            cancellation: CancellationToken::new(),
        }
    }

    fn cancel(self) {
        self.cancellation.cancel();
    }

    async fn cancelled(&self) {
        self.cancellation.cancelled().await;
    }
}

impl std::fmt::Debug for IdleShutdownTimer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IdleShutdownTimer").finish_non_exhaustive()
    }
}

/// Side effect selected after daemon session state changes.
#[derive(Debug, Clone)]
pub(super) enum DaemonSessionsEffect {
    None,
    CancelIdleShutdown { timer: IdleShutdownTimer },
    ScheduleIdleShutdown { timer: IdleShutdownTimer },
}

impl PartialEq for DaemonSessionsEffect {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for DaemonSessionsEffect {}

impl DaemonSessionsEffect {
    pub(super) fn apply(self, idle_shutdown: Option<&SessionIdleShutdown>) {
        match self {
            Self::None => {}
            Self::CancelIdleShutdown { timer } => timer.cancel(),
            Self::ScheduleIdleShutdown { timer } => {
                if let Some(idle_shutdown) = idle_shutdown {
                    idle_shutdown.schedule(timer);
                }
            }
        }
    }
}
