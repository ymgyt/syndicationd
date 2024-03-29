use std::{sync::Arc, time::Duration};

use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use synd_feed::feed::{
    cache::{CacheConfig, CacheLayer},
    parser::FeedService,
};

use crate::{
    args::{self, KvsdOptions, TlsOptions},
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
    ) -> anyhow::Result<Self> {
        let KvsdOptions {
            kvsd_host,
            kvsd_port,
            kvsd_username,
            kvsd_password,
        } = kvsd;
        let kvsd = KvsdClient::connect(
            kvsd_host,
            kvsd_port,
            kvsd_username,
            kvsd_password,
            Duration::from_secs(10),
        )
        .await?;

        let feed_service = FeedService::new(config::USER_AGENT, 10 * 1024 * 1024);
        let cache_feed_service = CacheLayer::with(
            feed_service,
            CacheConfig::default()
                .with_max_cache_size(100 * 1024 * 1024)
                .with_time_to_live(Duration::from_secs(60 * 60 * 3)),
        );

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
