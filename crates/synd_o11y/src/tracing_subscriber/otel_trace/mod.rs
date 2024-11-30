use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{BatchConfig, Sampler, Tracer},
    Resource,
};
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{registry::LookupSpan, Layer};

pub fn layer<S>(
    endpoint: impl Into<String>,
    resource: Resource,
    sampler_ratio: f64,
    batch_config: BatchConfig,
) -> impl Layer<S>
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    OpenTelemetryLayer::new(init_tracer(endpoint, resource, sampler_ratio, batch_config))
}

#[expect(clippy::needless_pass_by_value)]
fn init_tracer(
    endpoint: impl Into<String>,
    resource: Resource,
    sampler_ratio: f64,
    // TODO: how to use BatchConfig after 0.27 ?
    _batch_config: BatchConfig,
) -> Tracer {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()
        .unwrap();

    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_resource(resource)
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            sampler_ratio,
        ))))
        .with_batch_exporter(exporter, runtime::Tokio)
        .build();

    // > It would now be the responsibility of users to set it by calling global::set_tracer_provider(tracer_provider.clone());
    //  https://github.com/open-telemetry/opentelemetry-rust/blob/main/opentelemetry-otlp/CHANGELOG.md#v0170
    global::set_tracer_provider(provider.clone());

    provider.tracer("tracing-opentelemetry")
}

#[cfg(test)]
mod tests {
    use std::{net::SocketAddr, time::Duration};

    use opentelemetry::KeyValue;
    use opentelemetry_proto::tonic::{
        collector::trace::v1::{
            trace_service_server::{TraceService, TraceServiceServer},
            ExportTraceServiceRequest, ExportTraceServiceResponse,
        },
        trace::v1::{span::SpanKind, status::StatusCode, Status},
    };
    use opentelemetry_sdk::trace::BatchConfigBuilder;
    use tokio::sync::mpsc::UnboundedSender;
    use tonic::transport::Server;
    use tracing::dispatcher;
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    use super::*;

    type Request = tonic::Request<ExportTraceServiceRequest>;

    struct DumpTraces {
        tx: UnboundedSender<Request>,
    }

    #[tonic::async_trait]
    impl TraceService for DumpTraces {
        async fn export(
            &self,
            request: tonic::Request<ExportTraceServiceRequest>,
        ) -> Result<tonic::Response<ExportTraceServiceResponse>, tonic::Status> {
            self.tx.send(request).unwrap();

            Ok(tonic::Response::new(ExportTraceServiceResponse {
                partial_success: None, // means success
            }))
        }
    }

    #[tracing::instrument(fields(
        otel.name = "f1_custom",
        otel.kind = "Client",
    ) )]
    fn f1() {
        f2();
    }
    #[tracing::instrument(fields(
        otel.name = "f2_custom",
        otel.kind = "Server",
    ))]
    fn f2() {
        tracing::error!("error_f2");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn layer_test() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let dump = TraceServiceServer::new(DumpTraces { tx });
        let addr: SocketAddr = ([127, 0, 0, 1], 48100).into();
        let server = Server::builder().add_service(dump).serve(addr);
        let _server = tokio::task::spawn(server);
        let resource = resource();
        let config = BatchConfigBuilder::default()
            // The default interval is 5 seconds, which slows down the test
            .with_scheduled_delay(Duration::from_millis(10))
            .build();
        let layer = layer("https://localhost:48100", resource.clone(), 1.0, config);
        let subscriber = Registry::default().with(layer);
        let dispatcher = tracing::Dispatch::new(subscriber);

        dispatcher::with_default(&dispatcher, || {
            f1();
        });

        let req = rx.recv().await.unwrap().into_inner();
        assert_eq!(req.resource_spans.len(), 1);

        let resource_span = req.resource_spans[0].clone();
        insta::with_settings!({
            description => "trace resource",
        }, {
            insta::assert_yaml_snapshot!("layer_test_trace_resource", req.resource_spans[0].resource);
        });

        let [f2_span, f1_span] = resource_span.scope_spans[0].spans.as_slice() else {
            panic!()
        };

        assert_eq!(&f2_span.name, "f2_custom");
        assert_eq!(&f1_span.name, "f1_custom");
        assert_eq!(f2_span.parent_span_id, f1_span.span_id);
        assert_eq!(f2_span.trace_id, f1_span.trace_id);
        assert_eq!(f2_span.kind, SpanKind::Server as i32);
        assert_eq!(f1_span.kind, SpanKind::Client as i32);
        assert_eq!(
            f2_span.status,
            Some(Status {
                message: String::new(),
                code: StatusCode::Error as i32,
            })
        );
        assert_eq!(
            f1_span.status,
            Some(Status {
                message: String::new(),
                code: StatusCode::Unset as i32,
            })
        );
        assert_eq!(f2_span.events.len(), 1);
        assert_eq!(f2_span.attributes.len(), 7);
    }

    fn resource() -> Resource {
        Resource::new([KeyValue::new("service.name", "test")])
    }
}
