use synd_o11y::{opentelemetry::OpenTelemetryGuard, tracing_subscriber::otel_metrics};
use tracing::{error, info};

use synd_api::{args, config, dependency::Dependency, serve::listen_and_serve};

fn init_tracing() -> Option<OpenTelemetryGuard> {
    use synd_o11y::{
        opentelemetry::init_propagation,
        tracing_subscriber::{audit, otel_log, otel_trace},
    };
    use tracing_subscriber::{
        filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt as _, Layer as _,
        Registry,
    };

    let color = {
        use supports_color::Stream;
        supports_color::on(Stream::Stdout).is_some()
    };
    let show_src = true;
    let show_target = !show_src;

    let (opentelemetry_layers, guard) = {
        match std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").ok() {
            None => (None, None),
            Some(endpoint) if endpoint.is_empty() => (None, None),
            Some(endpoint) => {
                let resource =
                    synd_o11y::opentelemetry::resource(config::NAME, config::VERSION, "local");

                let trace_layer = otel_trace::layer(resource.clone());
                let log_layer = otel_log::layer(endpoint, resource.clone());
                let metrics_layer = otel_metrics::layer(resource);

                (
                    Some(trace_layer.and_then(log_layer).and_then(metrics_layer)),
                    Some(synd_o11y::opentelemetry::OpenTelemetryGuard),
                )
            }
        }
    };

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(color)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(show_src)
                .with_line_number(show_src)
                .with_target(show_target)
                .and_then(opentelemetry_layers)
                .with_filter(
                    EnvFilter::try_from_default_env()
                        .or_else(|_| EnvFilter::try_new("info"))
                        .unwrap()
                        .add_directive(audit::Audit::directive()),
                ),
        )
        .with(audit::layer())
        .init();

    // Set text map propagator globally
    init_propagation();

    guard
}

#[tokio::main]
async fn main() {
    let args = args::parse();

    let _guard = init_tracing();

    let dep = Dependency::new(args.kvsd).await.unwrap();

    info!(version = config::VERSION, "Runinng...");

    if let Err(err) = listen_and_serve(dep).await {
        error!("{err:?}");
        std::process::exit(1);
    }
}
