use std::{path::PathBuf, sync::Once, time::Duration};

use chrono::{DateTime, Utc};
use futures_util::future;
use octocrab::Octocrab;
use ratatui::backend::TestBackend;
use synd_api::{
    cli::{CacheOptions, ServeOptions, TlsOptions},
    client::github::GithubClient,
    dependency::Dependency,
    repository::{SubscriptionRepository, sqlite::DbPool, types::FeedSubscription},
    shutdown::Shutdown,
};
use synd_auth::{
    device_flow::{DeviceFlow, provider},
    jwt,
};
use synd_feed::types::{Category, Requirement};
pub use synd_term::integration::event_stream;
use synd_term::{
    application::{
        Application, Authenticator, Cache, Clock, Config, DeviceFlows, JwtService, SystemClock,
    },
    auth::Credential,
    client::{github::GithubClient as TermGithubClient, synd_api::Client},
    config::Categories,
    interact::mock::MockInteractor,
    terminal::Terminal,
    types::Time,
    ui::theme::Theme,
};
use synd_test::temp_dir;
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::EnvFilter;
use url::Url;

struct DummyClock(Time);

impl Clock for DummyClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
}

#[derive(Clone)]
pub struct TestCase {
    pub mock_port: u16,
    pub synd_api_port: u16,
    pub kvsd_port: u16,
    pub kvsd_root_dir: PathBuf,
    pub terminal_col_row: (u16, u16),
    pub config: Config,
    pub device_flow_case: &'static str,
    pub cache_dir: PathBuf,
    pub log_path: PathBuf,
    pub now: Option<Time>,

    pub login_credential: Option<Credential>,
    pub interactor_buffer_fn: Option<fn(&TestCase) -> Vec<String>>,
}

pub fn test_config() -> Config {
    Config::default().with_idle_timer_interval(Duration::from_millis(1000))
}

impl Default for TestCase {
    fn default() -> Self {
        Self {
            mock_port: 0,
            synd_api_port: 0,
            kvsd_port: 0,
            kvsd_root_dir: synd_test::temp_dir().keep(),
            terminal_col_row: (120, 30),
            config: test_config(),
            device_flow_case: "case1",
            cache_dir: temp_dir().keep(),
            log_path: temp_dir().keep().join("synd.log"),
            now: None,

            login_credential: None,
            interactor_buffer_fn: None,
        }
    }
}

impl TestCase {
    pub fn already_logined(self) -> Self {
        let cred = Credential::Github {
            access_token: "dummy_gh_token".into(),
        };
        self.with_credential(cred)
    }

    pub fn with_credential(mut self, cred: Credential) -> Self {
        self.login_credential = Some(cred);
        self
    }

    pub async fn run_api(&self) -> anyhow::Result<()> {
        let TestCase {
            mock_port,
            synd_api_port,
            kvsd_port,
            kvsd_root_dir,
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
            serve_api(mock_port, synd_api_port, kvsd_port, kvsd_root_dir).await?;
        }

        Ok(())
    }

    pub async fn init_app(&self) -> anyhow::Result<Application> {
        let TestCase {
            mock_port,
            synd_api_port,
            terminal_col_row: (term_col, term_row),
            config,
            device_flow_case,
            cache_dir,
            login_credential,
            interactor_buffer_fn,
            now,
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
                        .with_device_authorization_endpoint(Url::parse(&format!(
                            "http://localhost:{mock_port}/{device_flow_case}/github/login/device/code",
                        )).unwrap())
                        .with_token_endpoint(
                            Url::parse(&format!("http://localhost:{mock_port}/{device_flow_case}/github/login/oauth/access_token")).unwrap()),
                ),
                google: DeviceFlow::new(provider::Google::new("dummy", "dummy")
                    .with_device_authorization_endpoint(Url::parse(&format!("http://localhost:{mock_port}/{device_flow_case}/google/login/device/code")).unwrap())
                    .with_token_endpoint(Url::parse(&format!("http://localhost:{mock_port}/{device_flow_case}/google/login/oauth/access_token")).unwrap())
                ),
            };
            let jwt_service = {
                // client_id is used for verify jwt
                let google_jwt_service = jwt::google::JwtService::new(
                    synd_test::jwt::DUMMY_GOOGLE_CLIENT_ID,
                    synd_test::jwt::DUMMY_GOOGLE_CLIENT_ID,
                )
                .with_token_endpoint(
                    Url::parse(&format!("http://localhost:{mock_port}/google/oauth2/token"))
                        .unwrap(),
                );
                JwtService::new().with_google_jwt_service(google_jwt_service)
            };
            let authenticator = Authenticator::new()
                .with_device_flows(device_flows)
                .with_jwt_service(jwt_service);
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

            let interactor = {
                let buffer = if let Some(f) = interactor_buffer_fn {
                    f(self)
                } else {
                    Vec::new()
                };
                Box::new(MockInteractor::new().with_buffer(buffer))
            };

            let github_client = {
                let octo = Octocrab::builder()
                    .base_uri(format!("http://localhost:{mock_port}/github/rest"))?
                    .personal_token("dummpy_gh_pat".to_owned())
                    .build()
                    .unwrap();
                TermGithubClient::with(octo)
            };

            let clock: Box<dyn Clock> = {
                match now {
                    Some(now) => Box::new(DummyClock(now)),
                    None => Box::new(SystemClock),
                }
            };

            let mut app = Application::builder()
                .terminal(terminal)
                .client(client)
                .categories(Categories::default_toml())
                .config(config)
                .cache(cache)
                .theme(Theme::default())
                .authenticator(authenticator)
                .interactor(interactor)
                .github_client(github_client)
                .clock(clock)
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
        let show_code_location = std::env::var("SYND_LOG_LOCATION").ok().is_some();

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
    kvsd_root_dir: PathBuf,
) -> anyhow::Result<()> {
    let db = {
        let db = DbPool::connect(kvsd_root_dir.join(format!("{kvsd_port}_synd.db"))).await?;
        db.migrate().await?;

        if api_port == 6031 {
            // setup fixtures
            let test_user_id = "899cf3fa5afc0aa1";

            db.put_feed_subscription(FeedSubscription {
                user_id: test_user_id.into(),
                url: "http://localhost:6030/feed/twir_atom".try_into().unwrap(),
                requirement: Some(Requirement::Must),
                category: Some(Category::new("rust").unwrap()),
            })
            .await?;
            tokio::time::sleep(Duration::from_secs(1)).await;
            db.put_feed_subscription(FeedSubscription {
                user_id: test_user_id.into(),
                url: "http://localhost:6030/feed/o11y_news".try_into().unwrap(),
                requirement: Some(Requirement::Should),
                category: Some(Category::new("opentelemetry").unwrap()),
            })
            .await?;
        }

        db
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

    let mut dep = Dependency::new(
        db,
        tls_options,
        serve_options,
        cache_options,
        CancellationToken::new(),
    )
    .await
    .unwrap();

    {
        let github_endpoint: &'static str =
            format!("http://localhost:{oauth_provider_port}/github/graphql").leak();
        let github_client = GithubClient::new()?.with_endpoint(github_endpoint);
        let google_jwt =
            jwt::google::JwtService::new("dummy_google_client_id", "dummy_google_client_secret")
                .with_pem_endpoint(
                    Url::parse(&format!(
                        "http://localhost:{oauth_provider_port}/google/oauth2/v1/certs"
                    ))
                    .unwrap(),
                );

        dep.authenticator = dep
            .authenticator
            .with_github_client(github_client)
            .with_google_jwt(google_jwt);
    }

    let listener = TcpListener::bind(("localhost", api_port)).await?;

    tokio::spawn(synd_api::serve::serve(
        listener,
        dep,
        Shutdown::watch_signal(future::pending(), || {}),
    ));

    Ok(())
}

pub fn resize_event(columns: u16, rows: u16) -> crossterm::event::Event {
    crossterm::event::Event::Resize(columns, rows)
}

pub fn focus_gained_event() -> crossterm::event::Event {
    crossterm::event::Event::FocusGained
}

pub fn focus_lost_event() -> crossterm::event::Event {
    crossterm::event::Event::FocusLost
}
