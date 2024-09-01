use std::env;

use synd_kvs::config;

use crate::args::ObservabilityOptions;

mod args;

// TODO: instrument with otel
fn init_tracing(_options: &ObservabilityOptions) {
    // Install global collector configured based on KVS_LOG env var.
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env(
            config::env::LOG_DIRECTIVE,
        ))
        .with_target(true)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .with_thread_ids(true)
        .init();
}

#[tokio::main]
async fn main() {
    let args = match args::try_parse(env::args_os()) {
        Ok(args) => args,
        Err(err) => err.exit(),
    };

    init_tracing(&args.o11y);
}
