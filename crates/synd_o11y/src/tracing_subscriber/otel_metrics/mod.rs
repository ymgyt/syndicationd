use std::time::Duration;

use opentelemetry_sdk::{metrics::MeterProvider, runtime, Resource};
use tracing::{Metadata, Subscriber};
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::{filter::filter_fn, layer::Filter, registry::LookupSpan, Layer};

pub mod macros;

pub const METRICS_EVENT_TARGET: &str = "metrics";

pub fn metrics_event_filter<S: Subscriber>() -> impl Filter<S> {
    filter_fn(|metadata: &Metadata<'_>| metadata.target() != METRICS_EVENT_TARGET)
}

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
