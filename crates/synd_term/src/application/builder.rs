use std::sync::Arc;

use synd_support::html::{DefaultHtmlConverter, HtmlConverter};

use crate::{
    application::outbound::gh::GhClient,
    application::{Application, Authenticator, Cache, Clock, Config, FeedApi, FeedApiRef},
    config::Categories,
    interact::Interact,
    ui::theme::Theme,
};

pub struct ApplicationBuilder<
    FeedApi = (),
    Categories = (),
    Cache = (),
    Config = (),
    Theme = (),
    Interactor = (),
> {
    pub(super) feed_api: FeedApi,
    pub(super) categories: Categories,
    pub(super) cache: Cache,
    pub(super) config: Config,
    pub(super) theme: Theme,
    pub(super) interactor: Interactor,

    pub(super) authenticator: Option<Authenticator>,
    pub(super) gh_client: Option<GhClient>,
    pub(super) clock: Option<Box<dyn Clock>>,
    pub(super) html_converter: Arc<dyn HtmlConverter>,
    pub(super) authentication_required: bool,
}

impl Default for ApplicationBuilder {
    fn default() -> Self {
        Self {
            feed_api: (),
            categories: (),
            cache: (),
            config: (),
            theme: (),
            interactor: (),
            authenticator: None,
            gh_client: None,
            clock: None,
            html_converter: Arc::new(DefaultHtmlConverter),
            authentication_required: false,
        }
    }
}

impl<T2, T3, T4, T5, T6> ApplicationBuilder<(), T2, T3, T4, T5, T6> {
    #[must_use]
    pub fn feed_api(
        self,
        feed_api: impl FeedApi,
    ) -> ApplicationBuilder<FeedApiRef, T2, T3, T4, T5, T6> {
        ApplicationBuilder {
            feed_api: Arc::new(feed_api),
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            gh_client: self.gh_client,
            clock: self.clock,
            html_converter: self.html_converter,
            authentication_required: self.authentication_required,
        }
    }
}

impl<T2, T3, T4, T5, T6> ApplicationBuilder<T2, (), T3, T4, T5, T6> {
    #[must_use]
    pub fn categories(
        self,
        categories: Categories,
    ) -> ApplicationBuilder<T2, Categories, T3, T4, T5, T6> {
        ApplicationBuilder {
            feed_api: self.feed_api,
            categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            gh_client: self.gh_client,
            clock: self.clock,
            html_converter: self.html_converter,
            authentication_required: self.authentication_required,
        }
    }
}

impl<T2, T3, T4, T5, T6> ApplicationBuilder<T2, T3, (), T4, T5, T6> {
    #[must_use]
    pub fn cache(self, cache: Cache) -> ApplicationBuilder<T2, T3, Cache, T4, T5, T6> {
        ApplicationBuilder {
            feed_api: self.feed_api,
            categories: self.categories,
            cache,
            config: self.config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            gh_client: self.gh_client,
            clock: self.clock,
            html_converter: self.html_converter,
            authentication_required: self.authentication_required,
        }
    }
}

impl<T2, T3, T4, T5, T6> ApplicationBuilder<T2, T3, T4, (), T5, T6> {
    #[must_use]
    pub fn config(self, config: Config) -> ApplicationBuilder<T2, T3, T4, Config, T5, T6> {
        ApplicationBuilder {
            feed_api: self.feed_api,
            categories: self.categories,
            cache: self.cache,
            config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            gh_client: self.gh_client,
            clock: self.clock,
            html_converter: self.html_converter,
            authentication_required: self.authentication_required,
        }
    }
}

impl<T2, T3, T4, T5, T6> ApplicationBuilder<T2, T3, T4, T5, (), T6> {
    #[must_use]
    pub fn theme(self, theme: Theme) -> ApplicationBuilder<T2, T3, T4, T5, Theme, T6> {
        ApplicationBuilder {
            feed_api: self.feed_api,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            gh_client: self.gh_client,
            clock: self.clock,
            html_converter: self.html_converter,
            authentication_required: self.authentication_required,
        }
    }
}

impl<T2, T3, T4, T5, T6> ApplicationBuilder<T2, T3, T4, T5, T6, ()> {
    #[must_use]
    pub fn interactor(
        self,
        interactor: Box<dyn Interact>,
    ) -> ApplicationBuilder<T2, T3, T4, T5, T6, Box<dyn Interact>> {
        ApplicationBuilder {
            feed_api: self.feed_api,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            interactor,
            authenticator: self.authenticator,
            gh_client: self.gh_client,
            clock: self.clock,
            html_converter: self.html_converter,
            authentication_required: self.authentication_required,
        }
    }
}

impl<T2, T3, T4, T5, T6, T7> ApplicationBuilder<T2, T3, T4, T5, T6, T7> {
    #[must_use]
    pub fn authenticator(self, authenticator: Authenticator) -> Self {
        Self {
            authenticator: Some(authenticator),
            ..self
        }
    }

    #[must_use]
    pub fn gh_client(self, gh_client: GhClient) -> Self {
        Self {
            gh_client: Some(gh_client),
            ..self
        }
    }

    #[must_use]
    pub fn clock(self, clock: Box<dyn Clock>) -> Self {
        Self {
            clock: Some(clock),
            ..self
        }
    }

    #[must_use]
    pub fn html_converter(self, html_converter: Arc<dyn HtmlConverter>) -> Self {
        Self {
            html_converter,
            ..self
        }
    }

    /// Selects whether the constructed feed transport requires interactive authentication.
    #[must_use]
    pub fn authentication_required(self, authentication_required: bool) -> Self {
        Self {
            authentication_required,
            ..self
        }
    }
}

impl ApplicationBuilder<FeedApiRef, Categories, Cache, Config, Theme, Box<dyn Interact>> {
    #[must_use]
    pub fn build(self) -> Application {
        Application::new(self)
    }
}
