use tracing::{error, info};

use syndapi::{args, dependency::Dependency, serve::listen_and_serve};
use tracing_subscriber::Layer;

fn init_tracing() {
    use syndapi::serve::layer::audit;
    use tracing_subscriber::{
        filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt as _, Registry,
    };

    let color = {
        use supports_color::Stream;
        supports_color::on(Stream::Stdout).is_some()
    };

    let show_src = true;
    let show_target = !show_src;

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(color)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(show_src)
                .with_line_number(show_src)
                .with_target(show_target)
                .with_filter(
                    EnvFilter::try_from_default_env()
                        .or_else(|_| EnvFilter::try_new("info"))
                        .unwrap(),
                ),
        )
        .with(audit::layer())
        .init();
}

#[tokio::main]
async fn main() {
    let args = args::parse();

    init_tracing();

    let version = env!("CARGO_PKG_VERSION");
    let dep = Dependency::new(args.kvsd).await.unwrap();

    info!(version, "Runinng...");

    if let Err(err) = listen_and_serve(dep).await {
        error!("{err:?}");
        std::process::exit(1);
    }
}
