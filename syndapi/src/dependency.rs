use std::sync::Arc;

use synd::feed::parser::FeedService;

use crate::{
    args::KvsdOptions,
    config,
    gql::Resolver,
    persistence::{kvsd::KvsdClient, memory::MemoryDatastore},
    serve::auth::Authenticator,
    usecase::{authorize::Authorizer, MakeUsecase},
};

pub struct Dependency {
    pub make_usecase: MakeUsecase,
    pub authenticator: Authenticator,
    pub authorizer: Authorizer,
    pub resolver: Resolver,
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

        let datastore = Arc::new(MemoryDatastore::new());
        let feed_service = FeedService::new(config::USER_AGENT, 10 * 1024 * 1024);

        let make_usecase = MakeUsecase {
            datastore: datastore.clone(),
            fetch_feed: Arc::new(feed_service),
        };

        let resolver = Resolver {
            datastore: datastore.clone(),
        };

        let authenticator = Authenticator::new()?;
        let authorizer = Authorizer::new();

        Ok(Dependency {
            authenticator,
            make_usecase,
            authorizer,
            resolver,
        })
    }
}
