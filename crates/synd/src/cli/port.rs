use synd_client::Client;
use synd_runtime::Session;

use crate::{config::ConfigResolver, runtime::FeedRuntime};

pub(super) struct PortContext {
    pub(super) client: Client,
    _session: Session,
}

impl PortContext {
    pub(super) async fn new(config: &ConfigResolver) -> anyhow::Result<Self> {
        let session = FeedRuntime::new(config)?.acquire_session().await?;
        let client = session.client().clone();

        Ok(Self {
            client,
            _session: session,
        })
    }
}
