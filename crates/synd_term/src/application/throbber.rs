use std::time::Duration;

use tokio::time::{self, Interval, MissedTickBehavior};

/// The clock that wakes the UI to advance its request indicator.
pub(super) struct Throbber {
    interval: Interval,
}

impl Throbber {
    pub(super) fn new(period: Duration) -> Self {
        let mut interval = time::interval(period.max(Duration::from_millis(1)));
        interval.set_missed_tick_behavior(MissedTickBehavior::Skip);
        Self { interval }
    }

    pub(super) async fn tick(&mut self) {
        self.interval.tick().await;
    }
}
