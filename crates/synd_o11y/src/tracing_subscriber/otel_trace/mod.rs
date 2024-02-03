use opentelemetry_sdk::{
    runtime,
    trace::{BatchConfig, Sampler, Tracer},
    Resource,
};
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{registry::LookupSpan, Layer};

pub fn layer<S>(resource: Resource) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    OpenTelemetryLayer::new(init_tracer(resource))
}

fn init_tracer(resource: Resource) -> Tracer {
    // https://opentelemetry.io/docs/specs/otel/configuration/sdk-environment-variables/
    let sampler_ratio = std::env::var("OTEL_TRACES_SAMPLER_ARG")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.);

    // TODO: construct tonic transport channel
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_trace_config(
            opentelemetry_sdk::trace::Config::default()
                .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                    sampler_ratio,
                ))))
                .with_resource(resource),
        )
        .with_batch_config(BatchConfig::default())
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(runtime::Tokio)
        .unwrap()
}
