use std::sync::Arc;

use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use synd_feed::feed::{
    cache::{CacheConfig, CacheLayer},
    service::FeedService,
};
use tokio_util::sync::CancellationToken;

use crate::{
    cli::{self, CacheOptions, SqliteOptions, TlsOptions},
    config,
    monitor::Monitors,
    repository::sqlite::DbPool,
    serve::{ServeOptions, auth::Authenticator},
    usecase::{MakeUsecase, Runtime, authorize::Authorizer},
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
        sqlite: SqliteOptions,
        tls: TlsOptions,
        serve_options: cli::ServeOptions,
        cache: CacheOptions,
        ct: CancellationToken,
    ) -> anyhow::Result<Self> {
        let db = {
            let db = DbPool::connect(&sqlite.sqlite_db).await?;
            db.migrate().await?;
            db
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
            subscription_repo: Arc::new(db),
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
