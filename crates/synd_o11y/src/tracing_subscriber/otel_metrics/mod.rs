use std::time::Duration;

use opentelemetry::{
    global,
    metrics::{MeterProvider, Unit},
};
use opentelemetry_otlp::WithExportConfig as _;
use opentelemetry_sdk::{
    metrics::{
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
        Instrument, PeriodicReader, SdkMeterProvider, Stream, View,
    },
    runtime, Resource,
};
use tracing::{Metadata, Subscriber};
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::{filter::filter_fn, layer::Filter, registry::LookupSpan, Layer};

pub mod macros;

pub const METRICS_EVENT_TARGET: &str = "metrics";

pub fn metrics_event_filter<S: Subscriber>() -> impl Filter<S> {
    filter_fn(|metadata: &Metadata<'_>| metadata.target() != METRICS_EVENT_TARGET)
}

pub fn layer<S>(endpoint: impl Into<String>, resource: Resource) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    MetricsLayer::new(init_meter_provider(endpoint, resource))
}

fn init_meter_provider(endpoint: impl Into<String>, resource: Resource) -> impl MeterProvider {
    // Currently OtelpMetricPipeline does not provide a way to set up views.
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint)
        .build_metrics_exporter(
            Box::new(DefaultAggregationSelector::new()),
            Box::new(DefaultTemporalitySelector::new()),
        )
        .unwrap();

    let reader = PeriodicReader::builder(exporter, runtime::Tokio)
        .with_interval(Duration::from_secs(60))
        .build();

    let view = view();
    let meter_provider_builder = SdkMeterProvider::builder()
        .with_resource(resource)
        .with_reader(reader)
        .with_view(view);

    #[cfg(feature = "opentelemetry-stdout")]
    let stdout_reader = {
        let exporter = opentelemetry_stdout::MetricsExporterBuilder::default()
            .with_encoder(|writer, data| {
                serde_json::to_writer_pretty(writer, &data).unwrap();
                Ok(())
            })
            .build();
        PeriodicReader::builder(exporter, runtime::Tokio)
            .with_interval(Duration::from_secs(60))
            .build()
    };
    #[cfg(feature = "opentelemetry-stdout")]
    let meter_provider_builder = meter_provider_builder.with_reader(stdout_reader);

    let meter_provider = meter_provider_builder.build();

    global::set_meter_provider(meter_provider.clone());

    meter_provider
}

fn view() -> impl View {
    |instrument: &Instrument| -> Option<Stream> {
        tracing::debug!("{instrument:?}");

        match instrument.name.as_ref() {
            "graphql.duration" => Some(
                Stream::new()
                    .name(instrument.name.clone())
                    .description("graphql response duration")
                    // Currently we could not ingest metrics with Arregation::Base2ExponentialHistogram in grafanacloud
                    .aggregation(
                        opentelemetry_sdk::metrics::Aggregation::ExplicitBucketHistogram {
                            // https://opentelemetry.io/docs/specs/semconv/http/http-metrics/#http-server
                            boundaries: vec![
                                0.005, 0.01, 0.025, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0, 2.5,
                                5.0, 7.5, 10.0,
                            ],
                            record_min_max: false,
                        },
                    )
                    // https://opentelemetry.io/docs/specs/semconv/general/metrics/#instrument-units
                    .unit(Unit::new("s")),
            ),
            name => {
                tracing::debug!(name, "There is no explicit view");
                None
            }
        }
    }
}
