use std::{env, sync::Arc};

use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use synd_feed::feed::{
    cache::{CacheConfig, CacheLayer},
    service::FeedService,
};
use tokio_util::sync::CancellationToken;

use crate::{
    cli::{self, CacheOptions, LocalOptions, TlsOptions},
    config,
    monitor::Monitors,
    repository::sqlite::SqliteSubscriptionRepository,
    serve::{ServeOptions, auth::Authenticator},
    usecase::{MakeUsecase, Runtime, authorize::Authorizer},
};

pub struct Dependency {
    pub authenticator: Authenticator,
    pub runtime: Runtime,
    pub tls_config: Option<RustlsConfig>,
    pub serve_options: ServeOptions,
    pub monitors: Monitors,
}

impl Dependency {
    pub async fn new(
        db: SqliteSubscriptionRepository,
        tls: TlsOptions,
        serve_options: cli::ServeOptions,
        local: LocalOptions,
        cache: CacheOptions,
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
            db,
            serve_options,
            cache,
            ct,
            authenticator,
            tls_config,
        ))
    }

    pub fn new_local(
        db: SqliteSubscriptionRepository,
        serve_options: cli::ServeOptions,
        cache: CacheOptions,
        ct: CancellationToken,
        token: String,
    ) -> anyhow::Result<Self> {
        let authenticator = Authenticator::local(token)?;
        Ok(Self::build(
            db,
            serve_options,
            cache,
            ct,
            authenticator,
            None,
        ))
    }

    fn build(
        db: SqliteSubscriptionRepository,
        serve_options: cli::ServeOptions,
        cache: CacheOptions,
        ct: CancellationToken,
        authenticator: Authenticator,
        tls_config: Option<RustlsConfig>,
    ) -> Self {
        let cache_feed_service = {
            let CacheOptions {
                feed_cache_size_mb,
                feed_cache_ttl,
                feed_cache_refresh_interval,
            } = cache;
            let feed_service = FeedService::new(config::USER_AGENT, 10 * 1024 * 1024);
            let cache_feed_service = CacheLayer::with(
                feed_service,
                CacheConfig::default()
                    .with_max_cache_size(feed_cache_size_mb * 1024 * 1024)
                    .with_time_to_live(feed_cache_ttl),
            );
            let periodic_refresher = cache_feed_service
                .periodic_refresher()
                .with_emit_metrics(true);

            tokio::spawn(periodic_refresher.run(feed_cache_refresh_interval, ct));

            cache_feed_service
        };

        let make_usecase = MakeUsecase {
            subscription_repo: Arc::new(db),
            fetch_feed: Arc::new(cache_feed_service),
        };

        let authorizer = Authorizer::new();

        let runtime = Runtime::new(make_usecase, authorizer);

        let monitors = Monitors::new();

        Dependency {
            authenticator,
            runtime,
            tls_config,
            serve_options: serve_options.into(),
            monitors,
        }
    }
}
