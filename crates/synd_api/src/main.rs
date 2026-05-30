use std::{env, io};

use anyhow::Context as _;
use fdlimit::Outcome;
use synd_support::io::color::{ColorSupport, is_color_supported};
use synd_support::o11y::{
    opentelemetry::OpenTelemetryGuard, tracing_subscriber::initializer::TracingInitializer,
};
use tokio::io::AsyncReadExt;
use tracing::{error, info};

use synd_api::{
    cli::{self, Args, ObservabilityOptions},
    config,
    dependency::Dependency,
    serve::{auth::Authenticator, listen_and_serve},
    shutdown::Shutdown,
};
use synd_persistence::sqlite::{SqliteDatabase, SqliteFeedRegistryDb};
use synd_registry::FeedRegistryRuntime;

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
        local,
        lifecycle: _,
        o11y,
        feed_refresh,
        dry_run,
    }: Args,
    shutdown: Shutdown,
) -> anyhow::Result<()> {
    let db = SqliteDatabase::create_or_open(sqlite.sqlite_db).await?;
    db.migrate().await?;
    let db = SqliteFeedRegistryDb::new(db);

    let local_enabled = local.enabled;
    let authenticator = if local_enabled {
        let token =
            env::var(config::env::LOCAL_TOKEN).context("local mode requires SYND_LOCAL_TOKEN")?;
        Authenticator::local(token)?
    } else {
        Authenticator::new()?
    };
    let tls_config = tls.rustls_config(local_enabled).await?;
    let registry_runtime = FeedRegistryRuntime::start(
        db.clone(),
        db.event_journal(),
        feed_refresh.registry_config(),
        shutdown.cancellation_token(),
    );
    registry_runtime.reconcile_startup().await;
    let dep = Dependency::new(
        authenticator,
        registry_runtime.registry(),
        tls_config,
        serve,
    );

    info!(
        version = config::app::VERSION,
        otlp_endpoint=?o11y.otlp_endpoint,
        request_timeout=?dep.serve_options.timeout,
        request_body_limit_bytes=dep.serve_options.body_limit_bytes,
        concurrency_limit=?dep.serve_options.concurrency_limit,
        default_feed_refresh_interval_minutes=?feed_refresh.default_feed_refresh_interval.as_secs() / 60,
        "Runinng...",
    );

    dry_run.then(|| shutdown.shutdown());
    if shutdown.is_shutdown_requested() {
        info!("Shutdown requested before serving");
        return Ok(());
    }

    let result = listen_and_serve(dep, bind.into(), shutdown).await;
    // Keep the registry runtime alive while API handlers can access the registry;
    // dropping it shuts down the worker tasks behind the dependency graph.
    drop(registry_runtime);
    result
}

async fn shutdown_signal(shutdown_on_stdin_eof: bool) -> io::Result<()> {
    if shutdown_on_stdin_eof {
        tokio::select! {
            res = tokio::signal::ctrl_c() => res,
            res = wait_for_stdin_eof() => res,
        }
    } else {
        tokio::signal::ctrl_c().await
    }
}

async fn wait_for_stdin_eof() -> io::Result<()> {
    let mut stdin = tokio::io::stdin();
    // The buffer must be non-empty; reading into an empty slice may return immediately.
    let mut buf = [0_u8; 1];

    loop {
        if stdin.read(&mut buf).await? == 0 {
            info!("Received stdin EOF");
            return Ok(());
        }
    }
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
    let shutdown =
        Shutdown::watch_signal(shutdown_signal(args.lifecycle.shutdown_on_stdin_eof), || {});

    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();
    init_file_descriptor_limit();

    if let Err(err) = run(args, shutdown).await {
        error!("{err:?}");
        std::process::exit(1);
    }
}
