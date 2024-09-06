use std::{path::PathBuf, time::Duration};

use anyhow::anyhow;
use url::Url;

use crate::{
    application::{Cache, Clock, JwtService, SystemClock},
    auth,
    client::synd_api::Client,
};

pub(super) struct PortContext {
    pub(super) client: Client,
}

impl PortContext {
    pub(super) async fn new(endpoint: Url, cache_dir: PathBuf) -> anyhow::Result<Self> {
        let mut client = Client::new(endpoint, Duration::from_secs(10))?;
        let jwt_service = JwtService::new();
        let cache = Cache::new(cache_dir);
        let restore = auth::Restore {
            jwt_service: &jwt_service,
            cache: &cache,
            now: SystemClock.now(),
            persist_when_refreshed: false,
        };
        let credential = restore
            .restore()
            .await
            .map_err(|_| anyhow!("You are not authenticated, try login in first"))?;
        client.set_credential(credential);

        Ok(Self { client })
    }
}
