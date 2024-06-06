use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::{logs::LoggerProvider, runtime, Resource};
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, Layer};

pub fn layer<S>(endpoint: impl Into<String>, resource: Resource) -> (impl Layer<S>, LoggerProvider)
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    let provider = opentelemetry_otlp::new_pipeline()
        .logging()
        .with_log_config(opentelemetry_sdk::logs::Config::default().with_resource(resource))
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .install_batch(runtime::Tokio)
        .unwrap();

    (OpenTelemetryTracingBridge::new(&provider), provider)
}
