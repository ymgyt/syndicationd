use fdlimit::Outcome;
use synd_o11y::{
    opentelemetry::OpenTelemetryGuard, tracing_subscriber::otel_metrics::metrics_event_filter,
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

fn init_tracing(options: &ObservabilityOptions) -> OpenTelemetryGuard {
    use synd_o11y::{opentelemetry::init_propagation, tracing_subscriber::audit};
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

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(color)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(show_src)
                .with_line_number(show_src)
                .with_target(show_target)
                .with_filter(metrics_event_filter())
                .and_then(
                    options
                        .otlp_endpoint
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .map(|endpoint| {
                            synd_o11y::opentelemetry_layer(
                                endpoint,
                                config::NAME,
                                config::VERSION,
                                options.trace_sampler_ratio,
                            )
                        }),
                )
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

    synd_o11y::OpenTelemetryGuard
}

async fn run(
    Args {
        kvsd,
        bind,
        serve,
        tls,
        o11y,
    }: Args,
    shutdown: Shutdown,
) -> anyhow::Result<()> {
    let dep = Dependency::new(kvsd, tls, serve).await?;

    info!(
        version = config::VERSION,
        otlp_endpoint=?o11y.otlp_endpoint,
        request_timeout=?dep.serve_options.timeout,
        request_body_limit_bytes=dep.serve_options.body_limit_bytes,
        concurrency_limit=?dep.serve_options.concurrency_limit,
        "Runinng...",
    );

    listen_and_serve(dep, bind.into(), shutdown).await
}

fn init_file_descriptor_limit() {
    fdlimit::raise_fd_limit()
        .inspect(|outcome| {
            match outcome {
                Outcome::LimitRaised { from, to } => {
                    tracing::info!("Raise fd limit {from} to {to}");
                }

                Outcome::Unsupported => tracing::info!("Raise fd limit unsupported"),
            };
        })
        .ok();
}

#[tokio::main]
async fn main() {
    let args = args::parse();
    let _guard = init_tracing(&args.o11y);
    let shutdown = Shutdown::watch_signal();

    init_file_descriptor_limit();

    if let Err(err) = run(args, shutdown).await {
        if let Some(err) = err.downcast_ref::<ConnectKvsdFailed>() {
            error!("{err}: make sure kvsd is running");
        } else {
            error!("{err:?}");
        }
        std::process::exit(1);
    }
}
