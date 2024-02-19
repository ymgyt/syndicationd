use fdlimit::Outcome;
use synd_o11y::{
    opentelemetry::OpenTelemetryGuard,
    tracing_subscriber::otel_metrics::{self, metrics_event_filter},
};
use tracing::{error, info};

use synd_api::{
    args::{self, Args, ObservabilityOptions},
    config,
    dependency::Dependency,
    repository::kvsd::ConnectKvsdFailed,
    serve::listen_and_serve,
    shutdown::Shutdown,
};

fn init_tracing(options: &ObservabilityOptions) -> Option<OpenTelemetryGuard> {
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
    let show_src = options.show_code_location;
    let show_target = options.show_target;

    let (opentelemetry_layers, guard) = {
        match options.otlp_endpoint.as_deref() {
            None | Some("") => (None, None),
            Some(endpoint) => {
                let resource = synd_o11y::opentelemetry::resource(config::NAME, config::VERSION);

                let trace_layer =
                    otel_trace::layer(endpoint, resource.clone(), options.trace_sampler_ratio);
                let log_layer = otel_log::layer(endpoint, resource.clone());
                let metrics_layer = otel_metrics::layer(endpoint, resource);

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
                .with_filter(metrics_event_filter())
                .and_then(opentelemetry_layers)
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

    guard
}

async fn run(Args { kvsd, tls, o11y }: Args, shutdown: Shutdown) -> anyhow::Result<()> {
    let dep = Dependency::new(kvsd, tls).await?;

    info!(version = config::VERSION, otlp_endpoint=?o11y.otlp_endpoint, "Runinng...");

    listen_and_serve(dep, shutdown).await
}

#[tokio::main]
async fn main() {
    let args = args::parse();
    let _guard = init_tracing(&args.o11y);
    let shutdown = Shutdown::watch_signal();

    fdlimit::raise_fd_limit()
        .inspect(|outcome| match outcome {
            Outcome::LimitRaised { from, to } => tracing::info!("Raise fd limit {from} to {to}"),
            Outcome::Unsupported => tracing::info!("Raise fd limit unsupported"),
        })
        .ok();

    if let Err(err) = run(args, shutdown).await {
        if let Some(err) = err.downcast_ref::<ConnectKvsdFailed>() {
            error!("{err}: make sure kvsd is running");
        } else {
            error!("{err:?}");
        }
        std::process::exit(1);
    }
}
