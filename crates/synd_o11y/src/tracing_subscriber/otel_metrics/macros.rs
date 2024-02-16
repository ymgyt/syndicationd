#[macro_export]
macro_rules! metric {
    ($($tt:tt)* ) => { ::tracing::event!(
        target: $crate::tracing_subscriber::otel_metrics::METRICS_EVENT_TARGET,
        ::tracing::Level::INFO,
        $($tt)*
    );}
}
