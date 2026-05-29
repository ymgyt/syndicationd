use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::Context as _;
use rand::distr::{Alphanumeric, SampleString};
use synd_api::{
    cli::{FeedRefreshOptions, ServeOptions},
    dependency::Dependency,
    serve,
    shutdown::Shutdown,
};
use synd_client::{Client, ClientOptions};
use synd_persistence::sqlite::{SqliteDatabase, SqliteFeedRegistryDb};
use tokio::{net::TcpListener, task::JoinHandle};
use url::Url;

use crate::{Result, runtime::Config};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);
const READINESS_POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) struct LoopbackApi {
    pub(crate) client: Client,
    pub(crate) handle: LoopbackApiHandle,
}

pub(crate) struct LoopbackApiHandle {
    shutdown: Shutdown,
    task: JoinHandle<anyhow::Result<()>>,
}

struct SpawnedLoopbackApi {
    endpoint: Url,
    handle: LoopbackApiHandle,
}

#[derive(Clone, Debug)]
struct SessionToken(String);

struct LoopbackLaunch {
    sqlite_db: PathBuf,
    request_timeout: Duration,
    token: SessionToken,
    required_dirs: Vec<PathBuf>,
}

impl LoopbackApi {
    pub(crate) async fn start(config: &Config) -> Result<Self> {
        let token = SessionToken::generate();
        let launch = LoopbackLaunch::from_config(config, token.clone());
        let spawned = LoopbackSpawner::spawn(launch).await?;

        let mut client = Client::new(
            spawned.endpoint,
            ClientOptions::new(
                config.client().request_timeout(),
                config.client().user_agent(),
            ),
        )?;
        client.set_local_token(token.as_str())?;

        wait_until_ready(&client, &spawned.handle, config.session().acquire_timeout()).await?;

        Ok(Self {
            client,
            handle: spawned.handle,
        })
    }
}

impl LoopbackApiHandle {
    fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    pub(crate) async fn shutdown(mut self) -> Result<()> {
        self.shutdown.shutdown();

        match tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut self.task).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(err))) => tracing::warn!("loopback synd-api failed during shutdown: {err:?}"),
            Ok(Err(err)) => tracing::warn!("loopback synd-api task join failed: {err}"),
            Err(_) => {
                tracing::warn!("loopback synd-api did not stop within {SHUTDOWN_TIMEOUT:?}");
                self.task.abort();
            }
        }

        Ok(())
    }
}

impl Drop for LoopbackApiHandle {
    fn drop(&mut self) {
        if !self.task.is_finished() {
            self.shutdown.shutdown();
        }
    }
}

impl SessionToken {
    fn generate() -> Self {
        Self(Alphanumeric.sample_string(&mut rand::rng(), 64))
    }

    fn as_str(&self) -> &str {
        &self.0
    }

    fn into_string(self) -> String {
        self.0
    }
}

impl LoopbackLaunch {
    fn from_config(config: &Config, token: SessionToken) -> Self {
        let sqlite_db = config.database().sqlite_path().to_path_buf();
        let required_dirs = sqlite_db
            .parent()
            .map(|parent| vec![parent.to_path_buf()])
            .unwrap_or_default();

        Self {
            sqlite_db,
            request_timeout: config.client().request_timeout(),
            token,
            required_dirs,
        }
    }

    fn sqlite_db(&self) -> &Path {
        &self.sqlite_db
    }

    fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    fn required_dirs(&self) -> &[PathBuf] {
        &self.required_dirs
    }

    fn into_token(self) -> SessionToken {
        self.token
    }
}

struct LoopbackSpawner;

impl LoopbackSpawner {
    async fn spawn(launch: LoopbackLaunch) -> anyhow::Result<SpawnedLoopbackApi> {
        Self::create_required_dirs(launch.required_dirs())
            .context("failed to create runtime database parent directories")?;

        let listener = Self::bind_loopback_listener()
            .await
            .context("failed to bind runtime loopback listener")?;
        let endpoint = Url::parse(&format!(
            "http://{}",
            listener
                .local_addr()
                .context("failed to inspect runtime loopback listener address")?
        ))
        .context("failed to build runtime loopback endpoint")?;

        let db = Self::open_repository(launch.sqlite_db())
            .await
            .with_context(|| {
                format!(
                    "failed to open runtime sqlite database at {}",
                    launch.sqlite_db().display()
                )
            })?;
        let shutdown = Shutdown::manual(|| {
            tracing::info!("Gracefully shutdown loopback synd-api");
        });
        let dep = Dependency::new_local(
            db,
            ServeOptions {
                timeout: launch.request_timeout(),
                ..Default::default()
            },
            FeedRefreshOptions::default(),
            shutdown.cancellation_token(),
            launch.into_token().into_string(),
        )
        .await?;

        let task_shutdown = shutdown.clone();
        let task = tokio::spawn(async move { serve::serve(listener, dep, task_shutdown).await });

        Ok(SpawnedLoopbackApi {
            endpoint,
            handle: LoopbackApiHandle { shutdown, task },
        })
    }

    fn create_required_dirs(required_dirs: &[PathBuf]) -> anyhow::Result<()> {
        for dir in required_dirs {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    async fn bind_loopback_listener() -> anyhow::Result<TcpListener> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        TcpListener::bind(addr).await.map_err(anyhow::Error::from)
    }

    async fn open_repository(db_path: &Path) -> anyhow::Result<SqliteFeedRegistryDb> {
        let db = SqliteDatabase::create_or_open(db_path).await?;
        db.migrate().await?;
        Ok(SqliteFeedRegistryDb::new(db))
    }
}

async fn wait_until_ready(
    client: &Client,
    handle: &LoopbackApiHandle,
    acquire_timeout: Duration,
) -> anyhow::Result<()> {
    let deadline = Instant::now() + acquire_timeout;

    loop {
        if handle.is_finished() {
            anyhow::bail!("loopback synd-api exited before readiness");
        }

        if client.health().await.is_ok() {
            return Ok(());
        }

        if Instant::now() >= deadline {
            anyhow::bail!("loopback synd-api did not become ready within {acquire_timeout:?}");
        }

        tokio::time::sleep(READINESS_POLL_INTERVAL).await;
    }
}
