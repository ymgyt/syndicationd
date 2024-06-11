use std::{sync::Arc, time::Duration};

use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use synd_feed::feed::{
    cache::{CacheConfig, CacheLayer},
    service::FeedService,
};
use tokio_util::sync::CancellationToken;

use crate::{
    args::{self, CacheOptions, KvsdOptions, TlsOptions},
    config,
    monitor::Monitors,
    repository::kvsd::KvsdClient,
    serve::{auth::Authenticator, ServeOptions},
    usecase::{authorize::Authorizer, MakeUsecase, Runtime},
};

pub struct Dependency {
    pub authenticator: Authenticator,
    pub runtime: Runtime,
    pub tls_config: RustlsConfig,
    pub serve_options: ServeOptions,
    pub monitors: Monitors,
}

impl Dependency {
    pub async fn new(
        kvsd: KvsdOptions,
        tls: TlsOptions,
        serve_options: args::ServeOptions,
        cache: CacheOptions,
        ct: CancellationToken,
    ) -> anyhow::Result<Self> {
        let kvsd = {
            let KvsdOptions {
                kvsd_host,
                kvsd_port,
                kvsd_username,
                kvsd_password,
            } = kvsd;
            KvsdClient::connect(
                kvsd_host,
                kvsd_port,
                kvsd_username,
                kvsd_password,
                Duration::from_secs(10),
            )
            .await?
        };

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
            subscription_repo: Arc::new(kvsd),
            fetch_feed: Arc::new(cache_feed_service),
        };

        let authenticator = Authenticator::new()?;

        let authorizer = Authorizer::new();

        let runtime = Runtime::new(make_usecase, authorizer);

        let tls_config = RustlsConfig::from_pem_file(&tls.certificate, &tls.private_key)
            .await
            .with_context(|| format!("tls options: {tls:?}"))?;

        let monitors = Monitors::new();

        Ok(Dependency {
            authenticator,
            runtime,
            tls_config,
            serve_options: serve_options.into(),
            monitors,
        })
    }
}
