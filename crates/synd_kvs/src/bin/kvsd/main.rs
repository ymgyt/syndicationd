#[cfg(feature = "kvsd")]
#[tokio::main]
async fn main() {
    use std::env;
    use synd_kvs::{config, kvsd::cli};

    let _args = match cli::try_parse(env::args_os()) {
        Ok(args) => args,
        Err(err) => err.exit(),
    };

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_env(
            config::env::LOG_DIRECTIVE,
        ))
        .with_target(true)
        .with_timer(tracing_subscriber::fmt::time::ChronoLocal::rfc_3339())
        .with_thread_ids(true)
        .init();
}

#[cfg(not(feature = "kvsd"))]
fn main() {}
