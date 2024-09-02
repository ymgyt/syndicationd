use std::env;

use synd_kvsd::{
    args::{self, ObservabilityOptions},
    config,
};
use synd_o11y::{tracing_subscriber::otel_metrics::metrics_event_filter, OpenTelemetryGuard};
use synd_stdx::color::{is_color_supported, ColorSupport};

fn init_tracing(options: &ObservabilityOptions) -> Option<OpenTelemetryGuard> {
    use synd_o11y::{opentelemetry::init_propagation, tracing_subscriber::audit};
    use tracing_subscriber::{
        filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt as _, Layer as _,
        Registry,
    };

    let (otel_layer, otel_guard) = match options
        .otlp_endpoint
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|endpoint| {
            synd_o11y::opentelemetry_layer(
                endpoint,
                config::app::NAME,
                config::app::VERSION,
                options.trace_sampler_ratio,
            )
        }) {
        Some((otel_layer, otel_guard)) => (Some(otel_layer), Some(otel_guard)),
        _ => (None, None),
    };

    let ansi = is_color_supported() == ColorSupport::Supported;
    let show_src = options.show_code_location;
    let show_target = options.show_target;

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(ansi)
                .with_timer(fmt::time::ChronoLocal::rfc_3339())
                .with_file(show_src)
                .with_line_number(show_src)
                .with_target(show_target)
                .with_filter(metrics_event_filter())
                .and_then(otel_layer)
                .with_filter(
                    EnvFilter::try_from_env("SYND_LOG")
                        .or_else(|_| EnvFilter::try_new("info"))
                        .unwrap()
                        .add_directive(audit::Audit::directive()),
                ),
        )
        .with(audit::layer())
        .init();

    // Set text map propagator globally
    init_propagation();

    otel_guard
}

#[tokio::main]
async fn main() {
    let args = match args::try_parse(env::args_os()) {
        Ok(args) => args,
        Err(err) => err.exit(),
    };
    let _guard = init_tracing(&args.o11y);
}
