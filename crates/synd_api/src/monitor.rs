use std::time::Duration;

use synd_o11y::metric;
use tokio_metrics::{RuntimeMonitor, TaskMonitor};

pub struct Monitors {
    pub gql: TaskMonitor,
}

impl Monitors {
    pub fn new() -> Self {
        Self {
            gql: TaskMonitor::new(),
        }
    }

    pub async fn monitor(self, interval: Duration) {
        let handle = tokio::runtime::Handle::current();
        let runtime_monitor = RuntimeMonitor::new(&handle);
        let intervals = runtime_monitor.intervals().zip(self.gql.intervals());

        for (runtime_metrics, gql_metrics) in intervals {
            // Runtime metrics
            metric!(monotonic_counter.runtime.poll = runtime_metrics.total_polls_count);
            metric!(
                monotonic_counter.runtime.busy_duration =
                    runtime_metrics.total_busy_duration.as_secs_f64()
            );

            // Tasks poll metrics
            metric!(
                monotonic_counter.task.graphql.mean_poll_duration =
                    gql_metrics.mean_poll_duration().as_secs_f64()
            );
            metric!(
                monotonic_counter.task.graphql.mean_slow_poll_duration =
                    gql_metrics.mean_slow_poll_duration().as_secs_f64()
            );

            // Tasks schedule metrics
            metric!(
                monotonic_counter.task.graphql.mean_first_poll_delay =
                    gql_metrics.mean_first_poll_delay().as_secs_f64(),
            );
            metric!(
                monotonic_counter.task.graphql.mean_scheduled_duration =
                    gql_metrics.mean_scheduled_duration().as_secs_f64(),
            );

            // Tasks idle metrics
            metric!(
                monotonic_counter.task.graphql.mean_idle_duration =
                    gql_metrics.mean_idle_duration().as_secs_f64(),
            );

            tokio::time::sleep(interval).await;
        }
    }
}
