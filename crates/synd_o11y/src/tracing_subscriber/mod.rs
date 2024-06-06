use std::borrow::Cow;

use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, Layer};

use crate::OpenTelemetryGuard;

pub mod audit;
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

    let (log_layer, logger_provider) = otel_log::layer(endpoint.clone(), resource.clone());
    let guard = OpenTelemetryGuard { logger_provider };

    (
        otel_trace::layer(endpoint.clone(), resource.clone(), trace_sampler_ratio)
            .and_then(otel_metrics::layer(endpoint.clone(), resource.clone()))
            .and_then(log_layer),
        guard,
    )
}
