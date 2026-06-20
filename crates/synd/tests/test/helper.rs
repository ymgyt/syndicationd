use std::{net::SocketAddr, path::PathBuf, sync::Once, time::Duration};

use chrono::{DateTime, Utc};
use futures_util::future;
use synd_api::{
    client::github::GithubClient,
    dependency::Dependency,
    serve::{self, ServeOptions, auth::Authenticator as ApiAuthenticator},
    shutdown::Shutdown,
};
use synd_auth::{
    device_flow::{DeviceFlow, provider},
    jwt,
};
use synd_client::{Client, ClientOptions};
use synd_feed::types::FeedUrl;
use synd_feed::types::{Category, Requirement};
use synd_persistence::sqlite::{SqliteDatabase, SqliteFeedRegistryDb};
use synd_registry::{
    CommitTx, FeedRegistry, FeedRegistryDb, FeedSubscriptionAttrs, SubscriberId, SubscriptionKey,
    SubscriptionStore,
    crawl::policy::{CrawlPolicy, PollingInterval},
};
pub use synd_term::integration::{event_stream, new_test_terminal};
use synd_term::{
    application::{Application, Authenticator, Cache, Config, DeviceFlows, JwtService},
    auth::Credential,
    config::Categories,
    interact::mock::MockInteractor,
    ui::theme::Theme,
};
use synd_test::temp_dir;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use url::Url;

#[derive(Clone)]
pub struct TestCase {
    pub sqlite_root_dir: PathBuf,
    pub terminal_col_row: (u16, u16),
    pub config: Config,
    pub cache_dir: PathBuf,

    pub login_credential: Option<Credential>,
    pub subscriptions: Vec<SubscriptionSeed>,
}

pub fn test_config() -> Config {
    Config::default().with_idle_timer_interval(Duration::from_secs(1))
}

#[derive(Clone)]
pub struct SubscriptionSeed {
    pub feed_url: FeedUrl,
    pub requirement: Option<Requirement>,
    pub category: Option<Category<'static>>,
    pub crawl_policy: CrawlPolicy,
}

impl SubscriptionSeed {
    pub fn interval(
        feed_url: FeedUrl,
        requirement: Option<Requirement>,
        category: Option<Category<'static>>,
        interval: Duration,
    ) -> Self {
        Self {
            feed_url,
            requirement,
            category,
            crawl_policy: CrawlPolicy::interval(polling_interval(interval)),
        }
    }
}

impl Default for TestCase {
    fn default() -> Self {
        Self {
            sqlite_root_dir: synd_test::temp_dir().keep(),
            terminal_col_row: (120, 30),
            config: test_config(),
            cache_dir: temp_dir().keep(),

            login_credential: None,
            subscriptions: Vec::new(),
        }
    }
}

#[derive(Clone, Copy)]
struct ServiceAddrs {
    mock: SocketAddr,
    api: SocketAddr,
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

    async fn run_api(&self) -> anyhow::Result<ServiceAddrs> {
        let TestCase {
            sqlite_root_dir,
            subscriptions,
            ..
        } = self.clone();

        let mock_listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let mock = mock_listener.local_addr()?;
        let _mock_server = synd_test::mock::spawn(mock_listener);

        let api_listener = TcpListener::bind(("127.0.0.1", 0)).await?;
        let api = api_listener.local_addr()?;
        serve_api(mock, api_listener, sqlite_root_dir, subscriptions).await?;

        Ok(ServiceAddrs { mock, api })
    }

    pub async fn init_app(&self) -> anyhow::Result<Application> {
        let TestCase {
            terminal_col_row: (term_col, term_row),
            config,
            cache_dir,
            login_credential,
            ..
        } = self.clone();

        let services = self.run_api().await?;

        // Configure application
        let application = {
            let endpoint = format!("https://{}/graphql", services.api).parse().unwrap();
            let terminal = new_test_terminal(term_col, term_row);
            let client = Client::new(
                endpoint,
                ClientOptions::new(Duration::from_secs(10), "synd-integration-test"),
            )
            .unwrap();
            let device_flows = DeviceFlows {
                github: DeviceFlow::new(
                    provider::Github::new("dummy")
                        .with_device_authorization_endpoint(
                            Url::parse(&format!(
                                "http://{}/case1/github/login/device/code",
                                services.mock
                            ))
                            .unwrap(),
                        )
                        .with_token_endpoint(
                            Url::parse(&format!(
                                "http://{}/case1/github/login/oauth/access_token",
                                services.mock
                            ))
                            .unwrap(),
                        ),
                ),
                google: DeviceFlow::new(
                    provider::Google::new("dummy", "dummy")
                        .with_device_authorization_endpoint(
                            Url::parse(&format!(
                                "http://{}/case1/google/login/device/code",
                                services.mock
                            ))
                            .unwrap(),
                        )
                        .with_token_endpoint(
                            Url::parse(&format!(
                                "http://{}/case1/google/login/oauth/access_token",
                                services.mock
                            ))
                            .unwrap(),
                        ),
                ),
            };
            let jwt_service = {
                // client_id is used for verify jwt
                let google_jwt_service = jwt::google::JwtService::new(
                    synd_test::jwt::DUMMY_GOOGLE_CLIENT_ID,
                    synd_test::jwt::DUMMY_GOOGLE_CLIENT_ID,
                )
                .with_token_endpoint(
                    Url::parse(&format!("http://{}/google/oauth2/token", services.mock)).unwrap(),
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

            let interactor = Box::new(MockInteractor::new());

            let mut app = Application::builder()
                .terminal(terminal)
                .client(client)
                .categories(Categories::default_toml())
                .config(config)
                .cache(cache)
                .theme(Theme::default())
                .authenticator(authenticator)
                .interactor(interactor)
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
        // Initialize rustls crypto provider for integration tests
        let _ = rustls::crypto::ring::default_provider().install_default();

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

fn polling_interval(duration: Duration) -> PollingInterval {
    PollingInterval::try_from(duration).unwrap()
}

struct SubscriptionFixture {
    subscriber_id: SubscriberId,
    feed_url: FeedUrl,
    requirement: Option<Requirement>,
    category: Option<Category<'static>>,
    crawl_policy: CrawlPolicy,
    subscribed_at: DateTime<Utc>,
}

async fn seed_subscription(
    db: &SqliteFeedRegistryDb,
    fixture: SubscriptionFixture,
) -> anyhow::Result<()> {
    let mut tx = db.begin().await?;
    let subscription = SubscriptionKey::new(fixture.subscriber_id, fixture.feed_url.clone());
    let attrs = FeedSubscriptionAttrs {
        requirement: fixture.requirement,
        category: fixture.category,
        crawl_policy: fixture.crawl_policy,
    };
    tx.upsert_subscription(&subscription, attrs, fixture.subscribed_at)
        .await?;
    tx.commit().await?;

    Ok(())
}

pub async fn serve_api(
    oauth_provider_addr: SocketAddr,
    api_listener: TcpListener,
    sqlite_root_dir: PathBuf,
    subscriptions: Vec<SubscriptionSeed>,
) -> anyhow::Result<()> {
    let db = {
        let db = SqliteDatabase::create_or_open(sqlite_root_dir.join("synd.db")).await?;
        db.migrate().await?;
        let db = SqliteFeedRegistryDb::new(db);

        if !subscriptions.is_empty() {
            let subscriber_id = SubscriberId::new(synd_test::TEST_USER_ID);
            let subscribed_at = Utc::now();

            for subscription in subscriptions {
                let SubscriptionSeed {
                    feed_url,
                    requirement,
                    category,
                    crawl_policy,
                } = subscription;

                seed_subscription(
                    &db,
                    SubscriptionFixture {
                        subscriber_id: subscriber_id.clone(),
                        feed_url,
                        requirement,
                        category,
                        crawl_policy,
                        subscribed_at,
                    },
                )
                .await?;
            }
        }

        db
    };
    let serve_options = ServeOptions {
        timeout: Duration::from_secs(10),
        body_limit_bytes: 1024 * 2,
        concurrency_limit: 100,
        ..ServeOptions::default()
    };

    let shutdown = Shutdown::watch_signal(future::pending(), || {});
    let registry_config = synd_registry::FeedRegistryConfig {
        default_crawl_policy: CrawlPolicy::interval(polling_interval(Duration::from_hours(1))),
        ..synd_registry::FeedRegistryConfig::default()
    };
    let (registry, event_workers) =
        FeedRegistry::start(db.clone(), registry_config, shutdown.cancellation_token());
    let tls_config =
        serve::rustls_config_from_pem_files(synd_test::certificate(), synd_test::private_key())
            .await
            .unwrap();

    let mut dep = Dependency::new(
        ApiAuthenticator::new().unwrap(),
        registry,
        Some(tls_config),
        serve_options,
    );

    {
        let github_endpoint: &'static str =
            format!("http://{oauth_provider_addr}/github/graphql").leak();
        let github_client = GithubClient::new()?.with_endpoint(github_endpoint);
        let google_jwt =
            jwt::google::JwtService::new("dummy_google_client_id", "dummy_google_client_secret")
                .with_pem_endpoint(
                    Url::parse(&format!(
                        "http://{oauth_provider_addr}/google/oauth2/v1/certs"
                    ))
                    .unwrap(),
                );

        dep.authenticator = dep
            .authenticator
            .with_github_client(github_client)
            .with_google_jwt(google_jwt);
    }

    tokio::spawn(async move {
        let _event_workers = event_workers;
        synd_api::serve::serve(api_listener, dep, shutdown).await
    });

    Ok(())
}
