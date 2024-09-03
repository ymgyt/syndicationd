use std::{borrow::Cow, time::Duration};

use opentelemetry_sdk::{logs, trace};
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, Layer};

use crate::{tracing_subscriber::otel_metrics::metrics_event_filter, OpenTelemetryGuard};

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
    let trace_batch_config = trace::BatchConfigBuilder::default().build();
    let log_batch_config = logs::BatchConfigBuilder::default().build();
    let metrics_reader_interval = Duration::from_secs(60);

    let (log_layer, logger_provider) =
        otel_log::layer(endpoint.clone(), resource.clone(), log_batch_config);
    // Since metrics events are handled by the metrics layer and are not needed as logs, set afilter for them.
    let log_layer = log_layer.with_filter(metrics_event_filter());
    let guard = OpenTelemetryGuard { logger_provider };

    (
        otel_trace::layer(
            endpoint.clone(),
            resource.clone(),
            trace_sampler_ratio,
            trace_batch_config,
        )
        .and_then(otel_metrics::layer(
            endpoint.clone(),
            resource.clone(),
            metrics_reader_interval,
        ))
        .and_then(log_layer),
        guard,
    )
}
