use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::{runtime, Resource};
use tracing::Subscriber;
use tracing_subscriber::{registry::LookupSpan, Layer};

pub fn layer<S>(endpoint: impl Into<String>, resource: Resource) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_log_config(opentelemetry_sdk::logs::Config::default().with_resource(resource))
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(endpoint),
        )
        .install_batch(runtime::Tokio)
        .unwrap();

    let provider = opentelemetry::global::logger_provider();

    OpenTelemetryTracingBridge::new(&provider)
}
