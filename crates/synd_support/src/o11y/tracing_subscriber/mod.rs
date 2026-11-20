use std::{borrow::Cow, time::Duration};

use opentelemetry_sdk::{logs, trace};
use tracing::Subscriber;
use tracing_subscriber::{Layer, registry::LookupSpan};

use crate::o11y::{OpenTelemetryGuard, tracing_subscriber::otel_metrics::metrics_event_filter};

pub mod audit;
pub mod initializer;
pub mod otel_log;
pub mod otel_metrics;
pub mod otel_trace;

#[cfg(test)]
#[track_caller]
fn normalize_sdk_version(
    resource: &mut Option<opentelemetry_proto::tonic::resource::v1::Resource>,
) {
    use opentelemetry_proto::tonic::common::v1::any_value::Value;

    let resource = resource.as_mut().expect("missing exported resource");
    let version = resource
        .attributes
        .iter_mut()
        .find(|attribute| attribute.key == "telemetry.sdk.version")
        .and_then(|attribute| attribute.value.as_mut())
        .and_then(|value| value.value.as_mut());
    let Some(Value::StringValue(version)) = version else {
        panic!("missing string value for telemetry.sdk.version");
    };
    assert!(!version.trim().is_empty(), "empty telemetry.sdk.version");
    *version = "[SDK_VERSION]".into();
}

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
    let resource = crate::o11y::opentelemetry::resource(service_name, service_version);

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
        let metrics_reader_interval = Duration::from_mins(1);
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
