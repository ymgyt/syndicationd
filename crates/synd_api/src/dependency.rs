use std::sync::Arc;

use axum_server::tls_rustls::RustlsConfig;
use synd_registry::{FeedRegistry, sqlite::SqliteFeedRegistryDb};
use synd_support::html::{DefaultHtmlConverter, HtmlConverter};

use crate::{
    monitor::Monitors,
    serve::{ServeOptions, auth::Authenticator},
};

pub type LiveFeedRegistry = FeedRegistry<SqliteFeedRegistryDb>;

pub struct Dependency {
    pub authenticator: Authenticator,
    pub registry: LiveFeedRegistry,
    pub tls_config: Option<RustlsConfig>,
    pub serve_options: ServeOptions,
    pub monitors: Monitors,
    pub(crate) html_converter: Arc<dyn HtmlConverter>,
}

impl Dependency {
    pub fn new(
        authenticator: Authenticator,
        registry: LiveFeedRegistry,
        tls_config: Option<RustlsConfig>,
        serve_options: impl Into<ServeOptions>,
    ) -> Self {
        Self {
            authenticator,
            registry,
            tls_config,
            serve_options: serve_options.into(),
            monitors: Monitors::new(),
            html_converter: Arc::new(DefaultHtmlConverter),
        }
    }

    #[must_use]
    pub fn with_html_converter(mut self, converter: Arc<dyn HtmlConverter>) -> Self {
        self.html_converter = converter;
        self
    }
}
