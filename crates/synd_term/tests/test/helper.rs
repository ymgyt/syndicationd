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
use synd_auth::device_flow::{provider, DeviceFlow};
use synd_term::{
    application::{Application, Authenticator, Cache, Config, DeviceFlows},
    client::Client,
    config::Categories,
    terminal::Terminal,
    ui::theme::Theme,
};
use tokio::net::{TcpListener, TcpStream};
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct TestCase {
    pub oauth_provider_port: u16,
    pub synd_api_port: u16,
    pub kvsd_port: u16,
    pub terminal_col_row: (u16, u16),
    pub device_flow_case: &'static str,
    pub cache_dir: PathBuf,
}

impl TestCase {
    pub async fn init_app(&self) -> anyhow::Result<Application> {
        let TestCase {
            oauth_provider_port,
            synd_api_port,
            kvsd_port,
            terminal_col_row: (term_col, term_row),
            device_flow_case,
            cache_dir,
        } = self.clone();

        // Start mock oauth server
        {
            let addr = ("127.0.0.1", oauth_provider_port);
            let listener = TcpListener::bind(addr).await?;
            tokio::spawn(synd_test::mock::serve(listener));
        }

        // Start synd api server
        {
            serve_api(oauth_provider_port, synd_api_port, kvsd_port).await?;
        }

        // Configure application
        let application = {
            let endpoint = format!("https://localhost:{synd_api_port}/graphql")
                .parse()
                .unwrap();
            let terminal = new_test_terminal(term_col, term_row);
            let client = Client::new(endpoint, Duration::from_secs(10)).unwrap();
            let device_flows = DeviceFlows {
                github: DeviceFlow::new(
                    provider::Github::new("dummy")
                        .with_device_authorization_endpoint(format!(
                            "http://localhost:{oauth_provider_port}/{device_flow_case}/github/login/device/code",
                        ))
                        .with_token_endpoint(
                            format!("http://localhost:{oauth_provider_port}/{device_flow_case}/github/login/oauth/access_token"),
                        ),
                ),
                google: DeviceFlow::new(provider::Google::new("dummy", "dummy")),
            };
            let authenticator = Authenticator::new().with_device_flows(device_flows);
            let config = Config {
                idle_timer_interval: Duration::from_millis(1000),
                throbber_timer_interval: Duration::from_secs(3600), // disable throbber
                ..Default::default()
            };
            // to isolate the state for each test
            let cache = Cache::new(cache_dir);
            Application::with(terminal, client, Categories::default_toml(), config, cache)
                .with_theme(Theme::default())
                .with_authenticator(authenticator)
        };

        Ok(application)
    }
}

pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_line_number(true)
        .with_file(true)
        .with_target(false)
        .init();
}

pub fn new_test_terminal(width: u16, height: u16) -> Terminal {
    let backend = TestBackend::new(width, height);
    let terminal = ratatui::Terminal::new(backend).unwrap();
    Terminal::with(terminal)
}

pub async fn serve_api(
    oauth_provider_port: u16,
    api_port: u16,
    kvsd_port: u16,
) -> anyhow::Result<()> {
    let kvsd_options = KvsdOptions {
        kvsd_host: "localhost".into(),
        kvsd_port,
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
            format!("http://localhost:{oauth_provider_port}/github/graphql").leak();
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
