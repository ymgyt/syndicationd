use std::env;

use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use synd_feed::feed::service::FeedService;
use synd_persistence::sqlite::SqliteFeedRegistryStore;
use synd_registry::{
    FeedRegistry, FeedRegistryConfig, RefreshExecutor, RefreshExecutorHandle,
    RegistryEventPublisher, provider::SyndFeedProvider,
};
use tokio_util::sync::CancellationToken;

use crate::{
    cli::{self, FeedRefreshOptions, LocalOptions, TlsOptions},
    config,
    monitor::Monitors,
    serve::{ServeOptions, auth::Authenticator},
};

pub type AppFeedRegistry = FeedRegistry<SqliteFeedRegistryStore, SyndFeedProvider>;

pub struct Dependency {
    pub authenticator: Authenticator,
    pub registry: AppFeedRegistry,
    pub tls_config: Option<RustlsConfig>,
    pub serve_options: ServeOptions,
    pub monitors: Monitors,
}

impl Dependency {
    pub async fn new(
        store: SqliteFeedRegistryStore,
        tls: TlsOptions,
        serve_options: cli::ServeOptions,
        local: LocalOptions,
        feed_refresh: FeedRefreshOptions,
        ct: CancellationToken,
    ) -> anyhow::Result<Self> {
        let local_enabled = local.enabled;
        let authenticator = if local_enabled {
            let token = env::var(config::env::LOCAL_TOKEN)
                .context("local mode requires SYND_LOCAL_TOKEN")?;
            Authenticator::local(token)?
        } else {
            Authenticator::new()?
        };

        let tls_config = if local_enabled {
            None
        } else {
            let certificate = tls
                .certificate
                .as_ref()
                .context("tls cert is required unless local mode is enabled")?;
            let private_key = tls
                .private_key
                .as_ref()
                .context("tls key is required unless local mode is enabled")?;
            Some(
                RustlsConfig::from_pem_file(certificate, private_key)
                    .await
                    .with_context(|| format!("tls options: {tls:?}"))?,
            )
        };

        Ok(Self::build(
            store,
            serve_options,
            feed_refresh,
            ct,
            authenticator,
            tls_config,
        )
        .await)
    }

    pub async fn new_local(
        store: SqliteFeedRegistryStore,
        serve_options: cli::ServeOptions,
        feed_refresh: FeedRefreshOptions,
        ct: CancellationToken,
        token: String,
    ) -> anyhow::Result<Self> {
        let authenticator = Authenticator::local(token)?;
        Ok(Self::build(store, serve_options, feed_refresh, ct, authenticator, None).await)
    }

    #[allow(clippy::needless_pass_by_value)]
    async fn build(
        store: SqliteFeedRegistryStore,
        serve_options: cli::ServeOptions,
        feed_refresh: FeedRefreshOptions,
        ct: CancellationToken,
        authenticator: Authenticator,
        tls_config: Option<RustlsConfig>,
    ) -> Self {
        let registry_config = {
            let FeedRefreshOptions {
                default_feed_refresh_interval,
            } = feed_refresh;
            FeedRegistryConfig {
                default_refresh_interval: default_feed_refresh_interval,
                ..FeedRegistryConfig::default()
            }
        };
        let provider =
            SyndFeedProvider::new(FeedService::new(config::USER_AGENT, 10 * 1024 * 1024));
        let executor_handle = RefreshExecutorHandle::new();
        let events = RegistryEventPublisher::default();
        let registry = FeedRegistry::with_events(
            store.clone(),
            provider.clone(),
            executor_handle.clone(),
            registry_config,
            events.clone(),
        );
        let executor = RefreshExecutor::with_events(
            store.clone(),
            provider,
            executor_handle,
            registry_config,
            events,
        );
        tokio::spawn(executor.run(ct.clone()));
        let _ = registry
            .reconcile_now(synd_registry::ReconcileTrigger::Startup)
            .await
            .inspect_err(|err| tracing::warn!("startup feed reconcile failed: {err}"));
        spawn_scheduler(registry.clone(), registry_config, ct);

        let monitors = Monitors::new();

        Dependency {
            authenticator,
            registry,
            tls_config,
            serve_options: serve_options.into(),
            monitors,
        }
    }
}

fn spawn_scheduler(registry: AppFeedRegistry, config: FeedRegistryConfig, ct: CancellationToken) {
    tokio::spawn(async move {
        tokio::select! {
            () = ct.cancelled() => return,
            () = tokio::time::sleep(config.scheduler_tick_interval) => {}
        }

        let mut interval = tokio::time::interval(config.scheduler_tick_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                () = ct.cancelled() => break,
                _ = interval.tick() => {
                    if let Err(err) = registry
                        .reconcile_now(synd_registry::ReconcileTrigger::ScheduledTick)
                        .await
                    {
                        tracing::warn!("scheduled feed reconcile failed: {err}");
                    }
                }
            }
        }
    });
}
