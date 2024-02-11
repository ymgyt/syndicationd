use std::{sync::Arc, time::Duration};

use anyhow::Context;
use axum_server::tls_rustls::RustlsConfig;
use synd_feed::feed::{
    cache::{CacheConfig, CacheLayer},
    parser::FeedService,
};

use crate::{
    args::{KvsdOptions, TlsOptions},
    config,
    repository::kvsd::KvsdClient,
    serve::auth::Authenticator,
    usecase::{authorize::Authorizer, MakeUsecase, Runtime},
};

pub struct Dependency {
    pub authenticator: Authenticator,
    pub runtime: Runtime,
    pub tls_config: RustlsConfig,
}

impl Dependency {
    pub async fn new(kvsd: KvsdOptions, tls: TlsOptions) -> anyhow::Result<Self> {
        let KvsdOptions {
            host,
            port,
            username,
            password,
        } = kvsd;
        let kvsd =
            KvsdClient::connect(host, port, username, password, Duration::from_secs(10)).await?;

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

        Ok(Dependency {
            authenticator,
            runtime,
            tls_config,
        })
    }
}
