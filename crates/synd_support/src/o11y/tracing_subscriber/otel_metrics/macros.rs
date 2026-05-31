#[macro_export]
macro_rules! metric {
    ($($tt:tt)* ) => { $crate::o11y::tracing_subscriber::otel_metrics::__tracing_event!(
        target: $crate::o11y::tracing_subscriber::otel_metrics::METRICS_EVENT_TARGET,
        $crate::o11y::tracing_subscriber::otel_metrics::__TracingLevel::INFO,
        $($tt)*
    );}
}
