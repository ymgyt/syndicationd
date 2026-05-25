use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    time::Duration,
};

use synd_api::{
    cli::{FeedRefreshOptions, ServeOptions},
    dependency::Dependency,
    serve,
    shutdown::Shutdown,
};
use synd_persistence::sqlite::SqliteFeedRegistryStore;
use tokio::{net::TcpListener, task::JoinHandle};
use url::Url;

use crate::client::synd_api::Client;

use super::{
    config::LocalApiConfig, launch::LocalApiLaunch, readiness::wait_until_ready,
    token::LocalApiToken,
};

const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(3);

pub struct LocalApi {
    pub client: Client,
    pub runtime: LocalApiRuntime,
}

pub struct LocalApiRuntime {
    shutdown: Shutdown,
    task: JoinHandle<anyhow::Result<()>>,
}

struct SpawnedLocalApi {
    endpoint: Url,
    runtime: LocalApiRuntime,
}

struct LocalApiSpawner;

impl LocalApi {
    pub async fn start(config: LocalApiConfig) -> anyhow::Result<Self> {
        let token = LocalApiToken::generate();
        let launch = LocalApiLaunch::from_config(config, token.clone());
        let timeout = launch.timeout();
        let spawned = LocalApiSpawner::spawn(launch).await?;

        let mut client = Client::new(spawned.endpoint, timeout)?;
        client.set_local_token(token.as_str())?;

        let runtime = spawned.runtime;
        wait_until_ready(&client, &runtime).await?;

        Ok(Self { client, runtime })
    }
}

impl LocalApiRuntime {
    pub(super) fn is_finished(&self) -> bool {
        self.task.is_finished()
    }

    pub async fn shutdown(mut self) {
        self.shutdown.shutdown();

        match tokio::time::timeout(SHUTDOWN_TIMEOUT, &mut self.task).await {
            Ok(Ok(Ok(()))) => {}
            Ok(Ok(Err(err))) => tracing::warn!("local synd-api failed during shutdown: {err:?}"),
            Ok(Err(err)) => tracing::warn!("local synd-api task join failed: {err}"),
            Err(_) => {
                tracing::warn!("local synd-api did not stop within {SHUTDOWN_TIMEOUT:?}");
                self.task.abort();
            }
        }
    }
}

impl Drop for LocalApiRuntime {
    fn drop(&mut self) {
        if !self.task.is_finished() {
            self.shutdown.shutdown();
        }
    }
}

impl LocalApiSpawner {
    async fn spawn(launch: LocalApiLaunch) -> anyhow::Result<SpawnedLocalApi> {
        Self::create_required_dirs(launch.required_dirs())?;

        let listener = Self::bind_loopback_listener().await?;
        let endpoint = Url::parse(&format!("http://{}", listener.local_addr()?))?;

        let db = Self::open_repository(launch.sqlite_db()).await?;
        let shutdown = Shutdown::manual(|| {
            tracing::info!("Gracefully shutdown local synd-api");
        });
        let dep = Dependency::new_local(
            db,
            ServeOptions {
                timeout: launch.timeout(),
                ..Default::default()
            },
            FeedRefreshOptions::default(),
            shutdown.cancellation_token(),
            launch.into_token().into_string(),
        )
        .await?;

        let task_shutdown = shutdown.clone();
        let task = tokio::spawn(async move { serve::serve(listener, dep, task_shutdown).await });

        Ok(SpawnedLocalApi {
            endpoint,
            runtime: LocalApiRuntime { shutdown, task },
        })
    }

    fn create_required_dirs(required_dirs: &[std::path::PathBuf]) -> anyhow::Result<()> {
        for dir in required_dirs {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    async fn bind_loopback_listener() -> anyhow::Result<TcpListener> {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0);
        TcpListener::bind(addr).await.map_err(anyhow::Error::from)
    }

    async fn open_repository(db_path: &Path) -> anyhow::Result<SqliteFeedRegistryStore> {
        let db = SqliteFeedRegistryStore::create_or_open(db_path).await?;
        db.migrate().await?;
        Ok(db)
    }
}
