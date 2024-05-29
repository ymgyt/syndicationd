use std::{future::pending, path::PathBuf, time::Duration};

use futures_util::TryFutureExt;
use ratatui::backend::TestBackend;
use synd_api::{
    args::{CacheOptions, KvsdOptions, ServeOptions, TlsOptions},
    client::github::GithubClient,
    dependency::Dependency,
    repository::kvsd::KvsdClient,
    shutdown::Shutdown,
};
use synd_term::terminal::Terminal;
use tokio::net::{TcpListener, TcpStream};

pub fn new_test_terminal(width: u16, height: u16) -> Terminal {
    let backend = TestBackend::new(width, height);
    let terminal = ratatui::Terminal::new(backend).unwrap();
    Terminal::with(terminal)
}

pub async fn serve_api(mock_port: u16, api_port: u16) -> anyhow::Result<()> {
    let kvsd_options = KvsdOptions {
        kvsd_host: "localhost".into(),
        kvsd_port: 47379,
        kvsd_username: "test".into(),
        kvsd_password: "test".into(),
    };
    let tls_options = TlsOptions {
        certificate: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".dev")
            .join("self_signed_certs")
            .join("certificate.pem"),
        private_key: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".dev")
            .join("self_signed_certs")
            .join("private_key.pem"),
    };
    let serve_options = ServeOptions {
        timeout: Duration::from_secs(10),
        body_limit_bytes: 1024 * 2,
        concurrency_limit: 100,
    };
    let cache_options = CacheOptions {
        feed_cache_size_mb: 1,
        feed_cache_ttl: Duration::from_secs(60),
        feed_cache_refresh_interval: Duration::from_secs(3600),
    };

    let _kvsd_client = run_kvsd(kvsd_options.clone()).await.map(KvsdClient::new)?;

    let mut dep = Dependency::new(kvsd_options, tls_options, serve_options, cache_options)
        .await
        .unwrap();

    {
        let github_endpoint: &'static str =
            format!("http://localhost:{mock_port}/github/graphql").leak();
        let github_client = GithubClient::new()?.with_endpoint(github_endpoint);

        dep.authenticator = dep.authenticator.with_client(github_client);
    }

    let listener = TcpListener::bind(("localhost", api_port)).await?;

    tokio::spawn(synd_api::serve::serve(
        listener,
        dep,
        Shutdown::watch_signal(),
    ));

    Ok(())
}

pub async fn run_kvsd(
    KvsdOptions {
        kvsd_host,
        kvsd_port,
        kvsd_username,
        kvsd_password,
    }: KvsdOptions,
) -> anyhow::Result<kvsd::client::tcp::Client<TcpStream>> {
    let root_dir = temp_dir();
    let mut config = kvsd::config::Config::default();

    // Setup user credential.
    config.kvsd.users = vec![kvsd::core::UserEntry {
        username: kvsd_username,
        password: kvsd_password,
    }];
    config.server.set_disable_tls(&mut Some(true));

    // Test Server listen addr
    let addr = (kvsd_host, kvsd_port);

    let mut initializer = kvsd::config::Initializer::from_config(config);

    initializer.set_root_dir(root_dir.path());
    initializer.set_listener(TcpListener::bind(addr.clone()).await.unwrap());

    initializer.init_dir().await.unwrap();

    let _server_handler = tokio::spawn(initializer.run_kvsd(pending::<()>()));

    let handshake = async {
        loop {
            match kvsd::client::tcp::UnauthenticatedClient::insecure_from_addr(&addr.0, addr.1)
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

pub fn temp_dir() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}
