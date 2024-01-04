use clap::Parser;
use tracing::{error, info};

use syndapi::serve::listen_and_serve;

fn init_tracing() {
    use tracing_subscriber::{
        filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt as _, Registry,
    };

    let color = true;

    Registry::default()
        .with(
            fmt::Layer::new()
                .with_ansi(color)
                .with_timer(fmt::time::UtcTime::rfc_3339())
                .with_file(false)
                .with_line_number(false)
                .with_target(true),
        )
        .with(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("info"))
                .unwrap(),
        )
        .init();
}

#[derive(Parser, Debug)]
#[command(
    version,
    propagate_version = true,
    disable_help_subcommand = true,
    help_expected = true,
    about = "xxx"
)]
pub struct SyndApi {}

#[tokio::main]
async fn main() {
    init_tracing();

    let api = SyndApi::parse();
    let version = env!("CARGO_PKG_VERSION");

    info!(version, "Runinng...");

    if let Err(err) = listen_and_serve().await {
        error!("{err:?}");
        std::process::exit(1);
    }
}
