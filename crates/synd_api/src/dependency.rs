use axum_server::tls_rustls::RustlsConfig;
use synd_persistence::sqlite::SqliteFeedRegistryDb;
use synd_registry::FeedRegistry;

use crate::{
    monitor::Monitors,
    serve::{ServeOptions, auth::Authenticator},
    session::DaemonSessions,
};

pub type AppFeedRegistry = FeedRegistry<SqliteFeedRegistryDb>;

pub struct Dependency {
    pub authenticator: Authenticator,
    pub registry: AppFeedRegistry,
    pub tls_config: Option<RustlsConfig>,
    pub serve_options: ServeOptions,
    pub monitors: Monitors,
    pub sessions: DaemonSessions,
}

impl Dependency {
    pub fn new(
        authenticator: Authenticator,
        registry: AppFeedRegistry,
        tls_config: Option<RustlsConfig>,
        serve_options: impl Into<ServeOptions>,
    ) -> Self {
        Self {
            authenticator,
            registry,
            tls_config,
            serve_options: serve_options.into(),
            monitors: Monitors::new(),
            sessions: DaemonSessions::default(),
        }
    }
}
