use std::{io, path::PathBuf, sync::Once, time::Duration};

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
    auth::Credential,
    client::Client,
    config::Categories,
    interact::Interactor,
    terminal::Terminal,
    ui::theme::Theme,
};
use tokio::{net::TcpListener, sync::mpsc::UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing_subscriber::EnvFilter;

#[derive(Clone)]
pub struct TestCase {
    pub mock_port: u16,
    pub synd_api_port: u16,
    pub kvsd_port: u16,
    pub terminal_col_row: (u16, u16),
    pub device_flow_case: &'static str,
    pub cache_dir: PathBuf,

    pub login_credential: Option<Credential>,
    pub interactor_buffer: Option<String>,
}

impl Default for TestCase {
    fn default() -> Self {
        Self {
            mock_port: 0,
            synd_api_port: 0,
            kvsd_port: 0,
            terminal_col_row: (120, 30),
            device_flow_case: "case1",
            cache_dir: temp_dir().into_path(),

            login_credential: None,
            interactor_buffer: None,
        }
    }
}

impl TestCase {
    pub fn already_logined(mut self) -> Self {
        let cred = Credential::Github {
            access_token: "dummy_gh_token".into(),
        };
        self.login_credential = Some(cred);
        self
    }

    pub async fn run_api(&self) -> anyhow::Result<()> {
        let TestCase {
            mock_port,
            synd_api_port,
            kvsd_port,
            ..
        } = self.clone();

        // Start mock server
        {
            let addr = ("127.0.0.1", mock_port);
            let listener = TcpListener::bind(addr).await?;
            tokio::spawn(synd_test::mock::serve(listener));
        }

        // Start synd api server
        {
            serve_api(mock_port, synd_api_port, kvsd_port).await?;
        }

        Ok(())
    }

    pub async fn init_app(&self) -> anyhow::Result<Application> {
        let TestCase {
            mock_port,
            synd_api_port,
            terminal_col_row: (term_col, term_row),
            device_flow_case,
            cache_dir,
            login_credential,
            interactor_buffer,
            ..
        } = self.clone();

        self.run_api().await?;

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
                            "http://localhost:{mock_port}/{device_flow_case}/github/login/device/code",
                        ))
                        .with_token_endpoint(
                            format!("http://localhost:{mock_port}/{device_flow_case}/github/login/oauth/access_token"),
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

            let mut should_reload = false;
            // Configure logined state
            if let Some(cred) = login_credential {
                cache
                    .persist_credential(cred)
                    .expect("failed to save credential to cache");
                should_reload = true;
            }

            let mut app = Application::builder()
                .terminal(terminal)
                .client(client)
                .categories(Categories::default_toml())
                .config(config)
                .cache(cache)
                .theme(Theme::default())
                .authenticator(authenticator)
                .interactor(Interactor::new().with_buffer(interactor_buffer.unwrap_or_default()))
                .build();

            if should_reload {
                app.reload_cache().await.unwrap();
            }

            app
        };

        Ok(application)
    }
}

pub fn init_tracing() {
    static INIT_SUBSCRIBER: Once = Once::new();

    INIT_SUBSCRIBER.call_once(|| {
        let show_code_location = false;
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .with_line_number(show_code_location)
            .with_file(show_code_location)
            .with_target(true)
            .without_time()
            .init();
    });
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
        certificate: synd_test::certificate(),
        private_key: synd_test::private_key(),
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

    let _kvsd_client = synd_test::kvsd::run_kvsd(
        kvsd_options.kvsd_host.clone(),
        kvsd_options.kvsd_port,
        kvsd_options.kvsd_password.clone(),
        kvsd_options.kvsd_password.clone(),
    )
    .await
    .map(KvsdClient::new)?;

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

pub fn temp_dir() -> tempfile::TempDir {
    tempfile::TempDir::new().unwrap()
}

pub fn event_stream() -> (
    UnboundedSenderWrapper,
    UnboundedReceiverStream<io::Result<crossterm::event::Event>>,
) {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let tx = UnboundedSenderWrapper { inner: tx };
    let event_stream = UnboundedReceiverStream::new(rx);
    (tx, event_stream)
}

pub struct UnboundedSenderWrapper {
    inner: UnboundedSender<io::Result<crossterm::event::Event>>,
}

impl UnboundedSenderWrapper {
    pub fn send(&self, event: crossterm::event::Event) {
        self.inner.send(Ok(event)).unwrap();
    }
}
