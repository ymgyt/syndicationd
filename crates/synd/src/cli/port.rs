use synd_client::Client;
use synd_runtime::Session;
use tracing::warn;

use crate::{config::ConfigResolver, runtime::FeedRuntime};

pub(super) struct PortContext {
    pub(super) client: Client,
    session: Session,
}

impl PortContext {
    pub(super) async fn new(config: &ConfigResolver) -> anyhow::Result<Self> {
        let session = FeedRuntime::new(config)?.acquire_session().await?;
        let client = session.client().clone();

        Ok(Self { client, session })
    }

    pub(super) async fn finish<T>(self, result: anyhow::Result<T>) -> anyhow::Result<T> {
        if let Err(error) = self.session.close().await {
            warn!("Failed to close runtime session: {error}");
        }

        result
    }
}
