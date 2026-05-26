use std::path::PathBuf;

use anyhow::anyhow;

use crate::{
    application::{Cache, Clock, JwtService, SystemClock},
    auth,
    cli::BackendMode,
    client::synd_api::Client,
    config::ConfigResolver,
    local_api::{LocalApi, LocalApiConfig, LocalApiHandle},
};

pub(super) struct PortContext {
    pub(super) client: Client,
    _local_api_handle: Option<LocalApiHandle>,
}

pub(super) enum AuthMode {
    None,
    UserCredential { cache_dir: PathBuf },
}

impl PortContext {
    pub(super) async fn new(config: &ConfigResolver, auth_mode: AuthMode) -> anyhow::Result<Self> {
        match config.backend_mode() {
            BackendMode::Remote => Self::remote(config, auth_mode).await,
            BackendMode::Local => Self::local(config).await,
        }
    }

    async fn remote(config: &ConfigResolver, auth_mode: AuthMode) -> anyhow::Result<Self> {
        let mut client = Client::new(config.api_endpoint(), config.api_timeout())?;
        if let AuthMode::UserCredential { cache_dir } = auth_mode {
            Self::restore_user_credential(&mut client, cache_dir).await?;
        }

        Ok(Self {
            client,
            _local_api_handle: None,
        })
    }

    async fn local(config: &ConfigResolver) -> anyhow::Result<Self> {
        let local_api = LocalApi::start(LocalApiConfig {
            sqlite_db: config.sqlite_db(),
            timeout: config.api_timeout(),
        })
        .await?;

        Ok(Self {
            client: local_api.client,
            _local_api_handle: Some(local_api.handle),
        })
    }

    async fn restore_user_credential(
        client: &mut Client,
        cache_dir: PathBuf,
    ) -> anyhow::Result<()> {
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

        Ok(())
    }
}
