use tracing::{error, info};

use synd_api::{args, config, dependency::Dependency, serve::listen_and_serve};

fn init_tracing() {
    use synd_o11y::tracing_subscriber::{audit, otel_log};
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

    let otel_log_layer = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .filter(|endpoint| !endpoint.is_empty())
        .map(|endpoint| {
            let resource =
                synd_o11y::opentelemetry::resource(config::NAME, config::VERSION, "local");
            otel_log::layer(endpoint, resource)
        });

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(color)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(show_src)
                .with_line_number(show_src)
                .with_target(show_target)
                .and_then(otel_log_layer)
                .with_filter(
                    EnvFilter::try_from_default_env()
                        .or_else(|_| EnvFilter::try_new("info"))
                        .unwrap()
                        .add_directive(audit::Audit::directive()),
                ),
        )
        .with(audit::layer())
        .init();
}

#[tokio::main]
async fn main() {
    let args = args::parse();

    init_tracing();

    let dep = Dependency::new(args.kvsd).await.unwrap();

    info!(version = config::VERSION, "Runinng...");

    if let Err(err) = listen_and_serve(dep).await {
        error!("{err:?}");
        std::process::exit(1);
    }
}
