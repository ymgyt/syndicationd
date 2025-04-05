use std::{borrow::Cow, time::Duration};

use opentelemetry_sdk::{logs, trace};
use tracing::Subscriber;
use tracing_subscriber::{Layer, registry::LookupSpan};

use crate::{OpenTelemetryGuard, tracing_subscriber::otel_metrics::metrics_event_filter};

pub mod audit;
pub mod initializer;
pub mod otel_log;
pub mod otel_metrics;
pub mod otel_trace;

/// Return a composed layer that exports traces, metrics, and logs of opentelemetry.
/// The exported telemetry will be accompanied by a `Resource`.
pub fn opentelemetry_layer<S>(
    endpoint: impl Into<String>,
    service_name: impl Into<Cow<'static, str>>,
    service_version: impl Into<Cow<'static, str>>,
    trace_sampler_ratio: f64,
) -> (impl Layer<S>, OpenTelemetryGuard)
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let endpoint = endpoint.into();
    let resource = crate::opentelemetry::resource(service_name, service_version);

    let (trace_layer, tracer_provider) = {
        let trace_batch_config = trace::BatchConfigBuilder::default().build();
        otel_trace::layer(
            endpoint.clone(),
            resource.clone(),
            trace_sampler_ratio,
            trace_batch_config,
        )
    };

    let (metrics_layer, meter_provider) = {
        let metrics_reader_interval = Duration::from_secs(60);
        otel_metrics::layer(endpoint.clone(), resource.clone(), metrics_reader_interval)
    };

    let (log_layer, logger_provider) = {
        let log_batch_config = logs::BatchConfigBuilder::default().build();
        otel_log::layer(endpoint.clone(), resource.clone(), log_batch_config)
    };

    // Since metrics events are handled by the metrics layer and are not needed as logs, set a filter for them.
    let log_layer = log_layer.with_filter(metrics_event_filter());

    let guard = OpenTelemetryGuard {
        tracer_provider,
        meter_provider,
        logger_provider,
    };

    (
        trace_layer.and_then(metrics_layer).and_then(log_layer),
        guard,
    )
}
