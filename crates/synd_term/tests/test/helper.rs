use std::{future::pending, path::PathBuf, sync::Arc, time::Duration};

use axum_server::tls_rustls::RustlsConfig;
use futures_util::TryFutureExt;
use ratatui::backend::TestBackend;
use synd_api::{
    client::github::GithubClient,
    dependency::Dependency,
    repository::kvsd::KvsdClient,
    serve::{auth::Authenticator, ServeOptions},
    shutdown::Shutdown,
    usecase::{authorize::Authorizer, MakeUsecase, Runtime},
};
use synd_feed::feed::{cache::CacheLayer, parser::FeedService};
use synd_term::terminal::Terminal;
use tokio::net::{TcpListener, TcpStream};

pub fn new_test_terminal() -> Terminal {
    let backend = TestBackend::new(80, 20);
    let terminal = ratatui::Terminal::new(backend).unwrap();
    Terminal::with(terminal)
}

pub async fn serve_api(mock_port: u16, api_port: u16) -> anyhow::Result<()> {
    let github_endpoint: &'static str =
        format!("http://localhost:{mock_port}/github/graphql").leak();
    let github_client = GithubClient::new()?.with_endpoint(github_endpoint);
    let authenticator = Authenticator::new()?.with_client(github_client);

    let kvsd_client = run_kvsd().await.map(KvsdClient::new)?;
    let feed_service = FeedService::new("synd_term_test", 1024 * 1024);
    let feed_service = CacheLayer::new(feed_service);
    let make_usecase = MakeUsecase {
        subscription_repo: Arc::new(kvsd_client),
        fetch_feed: Arc::new(feed_service),
    };
    let authorizer = Authorizer::new();
    let runtime = Runtime::new(make_usecase, authorizer);
    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".dev")
            .join("self_signed_certs")
            .join("certificate.pem"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".dev")
            .join("self_signed_certs")
            .join("private_key.pem"),
    )
    .await?;
    let serve_options = ServeOptions {
        timeout: Duration::from_secs(10),
        body_limit_bytes: 1024 * 2,
        concurrency_limit: 100,
    };
    let dep = Dependency {
        authenticator,
        runtime,
        tls_config,
        serve_options,
    };
    let listener = TcpListener::bind(("localhost", api_port)).await?;

    tokio::spawn(synd_api::serve::serve(
        listener,
        dep,
        Shutdown::watch_signal(),
    ));

    Ok(())
}

pub async fn run_kvsd() -> anyhow::Result<kvsd::client::tcp::Client<TcpStream>> {
    let root_dir = temp_dir();
    let mut config = kvsd::config::Config::default();

    // Setup user credential.
    config.kvsd.users = vec![kvsd::core::UserEntry {
        username: "test".into(),
        password: "test".into(),
    }];
    config.server.set_disable_tls(&mut Some(true));

    // Test Server listen addr
    let addr = ("localhost", 47379);

    let mut initializer = kvsd::config::Initializer::from_config(config);

    initializer.set_root_dir(root_dir.path());
    initializer.set_listener(TcpListener::bind(addr).await.unwrap());

    initializer.init_dir().await.unwrap();

    let _server_handler = tokio::spawn(initializer.run_kvsd(pending::<()>()));

    let handshake = async {
        loop {
            match kvsd::client::tcp::UnauthenticatedClient::insecure_from_addr(addr.0, addr.1)
                .and_then(|client| client.authenticate("test", "test"))
                .await
            {
                Ok(client) => break client,
                Err(_) => tokio::time::sleep(Duration::from_millis(500)).await,
            }
        }
    };

    let client = tokio::time::timeout(Duration::from_secs(5), handshake).await?;

    Ok(client)
}

pub fn temp_dir() -> tempdir::TempDir {
    tempdir::TempDir::new("synd_term").unwrap()
}
