use std::time::Duration;

use opentelemetry_sdk::{metrics::MeterProvider, runtime, Resource};
use tracing::Subscriber;
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::{registry::LookupSpan, Layer};

pub fn layer<S>(resource: Resource) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    MetricsLayer::new(init_meter_provider(resource))
}

fn init_meter_provider(resource: Resource) -> MeterProvider {
    let provider = opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_resource(resource)
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .with_period(Duration::from_secs(60))
        .build()
        .unwrap();

    opentelemetry::global::set_meter_provider(provider.clone());

    provider
}
