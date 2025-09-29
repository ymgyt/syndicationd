use std::env;

use fdlimit::Outcome;
use synd_o11y::{
    opentelemetry::OpenTelemetryGuard, tracing_subscriber::initializer::TracingInitializer,
};
use synd_stdx::io::color::{ColorSupport, is_color_supported};
use tracing::{error, info};

use synd_api::{
    cli::{self, Args, ObservabilityOptions},
    config,
    dependency::Dependency,
    repository::kvsd::ConnectKvsdFailed,
    serve::listen_and_serve,
    shutdown::Shutdown,
};

fn init_tracing(options: &ObservabilityOptions) -> Option<OpenTelemetryGuard> {
    let ObservabilityOptions {
        show_code_location,
        show_target,
        otlp_endpoint,
        trace_sampler_ratio,
    } = options;

    TracingInitializer::default()
        .app_name(config::app::NAME)
        .app_version(config::app::VERSION)
        .otlp_endpoint(otlp_endpoint.clone())
        .trace_sampler_ratio(*trace_sampler_ratio)
        .enable_ansi(is_color_supported() == ColorSupport::Supported)
        .show_code_location(*show_code_location)
        .show_target(*show_target)
        .init()
}

async fn run(
    Args {
        sqlite,
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
        sqlite,
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
        .inspect(|outcome| match outcome {
            Outcome::LimitRaised { from, to } => {
                tracing::info!("Raise fd limit {from} to {to}");
            }

            Outcome::Unsupported => tracing::info!("Raise fd limit unsupported"),
        })
        .ok();
}

#[tokio::main]
async fn main() {
    let args = match cli::try_parse(env::args_os()) {
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
