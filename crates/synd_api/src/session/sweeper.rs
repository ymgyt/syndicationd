use std::time::Duration;

use tokio::{
    task::JoinHandle,
    time::{self, MissedTickBehavior},
};
use tokio_util::sync::CancellationToken;
use tracing::debug;

use super::DaemonSessions;

const MIN_SWEEP_INTERVAL: Duration = Duration::from_millis(100);

/// Daemon-wide controller that expires sessions whose leases are no longer renewed.
pub(crate) struct DaemonSessionSweeper {
    sessions: DaemonSessions,
    schedule: DaemonSessionSweepSchedule,
    shutdown: CancellationToken,
}

impl DaemonSessionSweeper {
    pub(crate) fn new(sessions: DaemonSessions, shutdown: CancellationToken) -> Self {
        let schedule = DaemonSessionSweepSchedule::from_interval(sessions.sweep_interval());

        Self {
            sessions,
            schedule,
            shutdown,
        }
    }

    pub(crate) fn spawn(self) -> DaemonSessionSweeperHandle {
        debug!(
            sweep_interval_ms = self.schedule.interval().as_millis(),
            "Started daemon session sweeper"
        );

        DaemonSessionSweeperHandle {
            task: tokio::spawn(self.run()),
        }
    }

    async fn run(self) {
        let mut interval = time::interval(self.schedule.interval());
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                () = self.shutdown.cancelled() => {
                    debug!("Stopped daemon session sweeper");
                    break;
                }
                _ = interval.tick() => {
                    let outcome = self.sessions.sweep_expired();
                    debug!(
                        expired_sessions = outcome.expired_session_count(),
                        active_sessions = outcome.active_sessions(),
                        "Daemon session sweeper tick"
                    );
                }
            }
        }
    }
}

/// Handle that keeps the daemon session sweeper task alive.
pub(crate) struct DaemonSessionSweeperHandle {
    task: JoinHandle<()>,
}

impl Drop for DaemonSessionSweeperHandle {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Sweep cadence used by the daemon session sweeper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DaemonSessionSweepSchedule {
    interval: Duration,
}

impl DaemonSessionSweepSchedule {
    fn from_interval(interval: Duration) -> Self {
        Self {
            interval: interval.max(MIN_SWEEP_INTERVAL),
        }
    }

    fn interval(self) -> Duration {
        self.interval
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{DaemonSessionSweepSchedule, MIN_SWEEP_INTERVAL};

    #[test]
    fn preserves_configured_sweep_interval() {
        let schedule = DaemonSessionSweepSchedule::from_interval(Duration::from_secs(5));

        assert_eq!(schedule.interval(), Duration::from_secs(5));
    }

    #[test]
    fn sweep_interval_has_minimum() {
        let schedule = DaemonSessionSweepSchedule::from_interval(Duration::ZERO);

        assert_eq!(schedule.interval(), MIN_SWEEP_INTERVAL);
    }
}
