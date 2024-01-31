use std::{sync::Arc, time::Duration};

use synd::feed::{
    cache::{CacheConfig, CacheLayer},
    parser::FeedService,
};

use crate::{
    args::KvsdOptions,
    config,
    persistence::{kvsd::KvsdClient, memory::MemoryDatastore},
    serve::auth::Authenticator,
    usecase::{authorize::Authorizer, MakeUsecase, Runtime},
};

pub struct Dependency {
    pub authenticator: Authenticator,
    pub runtime: Runtime,
}

impl Dependency {
    pub async fn new(kvsd: KvsdOptions) -> anyhow::Result<Self> {
        let KvsdOptions {
            host,
            port,
            username,
            password,
        } = kvsd;
        let _kvsd = KvsdClient::connect(host, port, username, password)
            .await
            .ok();

        let datastore = MemoryDatastore::new();

        let feed_service = FeedService::new(config::USER_AGENT, 10 * 1024 * 1024);
        let cache_feed_service = CacheLayer::with(
            feed_service,
            CacheConfig::default()
                .with_max_cache_size(100 * 1024 * 1024)
                .with_time_to_live(Duration::from_secs(60 * 60 * 3)),
        );

        let make_usecase = MakeUsecase {
            datastore: Arc::new(datastore),
            fetch_feed: Arc::new(cache_feed_service),
        };

        let authenticator = Authenticator::new()?;

        let authorizer = Authorizer::new();

        let runtime = Runtime::new(make_usecase, authorizer);

        Ok(Dependency {
            authenticator,
            runtime,
        })
    }
}
