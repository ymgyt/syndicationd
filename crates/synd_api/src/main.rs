use std::env;

use fdlimit::Outcome;
use synd_o11y::{
    opentelemetry::OpenTelemetryGuard, tracing_subscriber::otel_metrics::metrics_event_filter,
};
use synd_stdx::color::{is_color_supported, ColorSupport};
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

async fn run(
    Args {
        kvsd,
        bind,
        serve,
        tls,
        o11y,
        cache,
        dry_run,
    }: Args,
    shutdown: Shutdown,
) -> anyhow::Result<()> {
    let dep = Dependency::new(
        kvsd,
        tls,
        serve,
        cache.clone(),
        shutdown.cancellation_token(),
    )
    .await?;

    info!(
        version = config::app::VERSION,
        otlp_endpoint=?o11y.otlp_endpoint,
        request_timeout=?dep.serve_options.timeout,
        request_body_limit_bytes=dep.serve_options.body_limit_bytes,
        concurrency_limit=?dep.serve_options.concurrency_limit,
        feed_cache_ttl_minutes=?cache.feed_cache_ttl.as_secs() / 60,
        feed_cache_refresh_interval_minutes=?cache.feed_cache_refresh_interval.as_secs() / 60,
        "Runinng...",
    );

    dry_run.then(|| shutdown.shutdown());

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
    let args = match args::try_parse(env::args_os()) {
        Ok(args) => args,
        Err(err) => err.exit(),
    };
    let _guard = init_tracing(&args.o11y);
    let shutdown = Shutdown::watch_signal(tokio::signal::ctrl_c(), || {});

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
