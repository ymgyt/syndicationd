use std::time::Duration;

use synd_o11y::metric;
use tokio_metrics::{RuntimeMetrics, RuntimeMonitor, TaskMetrics, TaskMonitor};
use tokio_util::sync::CancellationToken;

struct Metrics {
    runtime_total_polls_count: u64,
    runtime_busy_duration_secs: f64,
    gql_mean_poll_duration_secs: f64,
    gql_mean_slow_poll_duration_secs: f64,
    gql_mean_first_poll_delay_secs: f64,
    gql_mean_scheduled_duration_secs: f64,
    gql_mean_idle_duration_secs: f64,
}

pub struct Monitors {
    gql: TaskMonitor,
}

impl Monitors {
    pub fn new() -> Self {
        Self {
            gql: TaskMonitor::new(),
        }
    }

    pub(crate) fn graphql_task_monitor(&self) -> TaskMonitor {
        self.gql.clone()
    }

    pub async fn emit_metrics(self, interval: Duration, ct: CancellationToken) {
        let handle = tokio::runtime::Handle::current();
        let runtime_monitor = RuntimeMonitor::new(&handle);
        let intervals = runtime_monitor.intervals().zip(self.gql.intervals());

        for (runtime_metrics, gql_metrics) in intervals {
            let Metrics {
                runtime_total_polls_count,
                runtime_busy_duration_secs,
                gql_mean_poll_duration_secs,
                gql_mean_slow_poll_duration_secs,
                gql_mean_first_poll_delay_secs,
                gql_mean_scheduled_duration_secs,
                gql_mean_idle_duration_secs,
            } = Self::collect_metrics(&runtime_metrics, &gql_metrics);

            // Runtime metrics
            metric!(monotonic_counter.runtime.poll = runtime_total_polls_count);
            metric!(monotonic_counter.runtime.busy_duration = runtime_busy_duration_secs);

            // Tasks poll metrics
            metric!(
                monotonic_counter.task.graphql.mean_poll_duration = gql_mean_poll_duration_secs
            );
            metric!(
                monotonic_counter.task.graphql.mean_slow_poll_duration =
                    gql_mean_slow_poll_duration_secs
            );

            // Tasks schedule metrics
            metric!(
                monotonic_counter.task.graphql.mean_first_poll_delay =
                    gql_mean_first_poll_delay_secs,
            );
            metric!(
                monotonic_counter.task.graphql.mean_scheduled_duration =
                    gql_mean_scheduled_duration_secs,
            );

            // Tasks idle metrics
            metric!(
                monotonic_counter.task.graphql.mean_idle_duration = gql_mean_idle_duration_secs,
            );

            tokio::select! {
                biased;
                // Make sure to respect cancellation
                () = ct.cancelled() => break,
                () = tokio::time::sleep(interval) => continue
            }
        }
    }

    fn collect_metrics(runtime_metrics: &RuntimeMetrics, gql_metrics: &TaskMetrics) -> Metrics {
        Metrics {
            runtime_total_polls_count: runtime_metrics.total_polls_count,
            runtime_busy_duration_secs: runtime_metrics.total_busy_duration.as_secs_f64(),
            gql_mean_poll_duration_secs: gql_metrics.mean_poll_duration().as_secs_f64(),
            gql_mean_slow_poll_duration_secs: gql_metrics.mean_slow_poll_duration().as_secs_f64(),
            gql_mean_first_poll_delay_secs: gql_metrics.mean_first_poll_delay().as_secs_f64(),
            gql_mean_scheduled_duration_secs: gql_metrics.mean_scheduled_duration().as_secs_f64(),
            gql_mean_idle_duration_secs: gql_metrics.mean_idle_duration().as_secs_f64(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing::{instrument::WithSubscriber, Event, Subscriber};
    use tracing_subscriber::{
        layer::{Context, SubscriberExt as _},
        registry::LookupSpan,
        Layer, Registry,
    };

    use super::*;

    struct TestLayer<F> {
        on_event: F,
    }

    impl<S, F> Layer<S> for TestLayer<F>
    where
        S: Subscriber + for<'span> LookupSpan<'span>,
        F: Fn(&Event<'_>) + 'static,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
            (self.on_event)(event);
        }
    }

    #[tokio::test]
    async fn emit_metrics() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let events_cloned = events.clone();
        let on_event = move |event: &Event<'_>| {
            let field = event.fields().next().unwrap().name();
            events_cloned.lock().unwrap().push(field);
        };
        let layer = TestLayer { on_event };
        let registry = Registry::default().with(layer);
        let m = Monitors::new();
        let ct = CancellationToken::new();
        ct.cancel();

        m.emit_metrics(Duration::from_millis(0), ct)
            .with_subscriber(registry)
            .await;

        let events = events.lock().unwrap().clone();
        insta::with_settings!({
            description => "metrics which monitor emits",
            omit_expression => true ,
        }, {
            insta::assert_yaml_snapshot!(events);
        });
    }
}
