use tracing::{error, info};

use syndapi::{
    args::{self, KvsdOptions},
    persistence::{kvsd::KvsdClient, Datastore},
    serve::{auth::Authenticator, listen_and_serve, Dependency},
};

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

async fn dependency(kvsd: KvsdOptions) -> anyhow::Result<Dependency> {
    let authenticator = Authenticator::new()?;
    let datastore = {
        let KvsdOptions {
            host,
            port,
            username,
            password,
        } = kvsd;
        let kvsd = KvsdClient::connect(host, port, username, password)
            .await
            .ok();
        Datastore::new(kvsd)?
    };

    Ok(Dependency {
        datastore,
        authenticator,
    })
}

#[tokio::main]
async fn main() {
    let args = args::parse();

    init_tracing();

    let version = env!("CARGO_PKG_VERSION");
    let dep = dependency(args.kvsd).await.unwrap();

    info!(version, "Runinng...");

    if let Err(err) = listen_and_serve(dep).await {
        error!("{err:?}");
        std::process::exit(1);
    }
}
