use crate::{
    application::outbound::github::GithubClient,
    application::{
        Application, Authenticator, Cache, Clock, Config, FeedApiRef, FeedApiSession, FeedBackend,
        SessPending, TermInit,
    },
    config::Categories,
    interact::Interact,
    terminal::Terminal,
    ui::theme::Theme,
};

pub struct ApplicationBuilder<
    Terminal = (),
    FeedApi = (),
    Categories = (),
    Cache = (),
    Config = (),
    Theme = (),
    Interactor = (),
> {
    pub(super) terminal: Terminal,
    pub(super) feed_api: FeedApi,
    pub(super) feed_api_session: FeedApiSession,
    pub(super) categories: Categories,
    pub(super) cache: Cache,
    pub(super) config: Config,
    pub(super) theme: Theme,
    pub(super) interactor: Interactor,

    pub(super) authenticator: Option<Authenticator>,
    pub(super) github_client: Option<GithubClient>,
    pub(super) clock: Option<Box<dyn Clock>>,
    pub(super) dry_run: bool,
}

impl Default for ApplicationBuilder {
    fn default() -> Self {
        Self {
            terminal: (),
            feed_api: (),
            feed_api_session: FeedApiSession::UserCredentialRequired,
            categories: (),
            cache: (),
            config: (),
            theme: (),
            interactor: (),
            authenticator: None,
            github_client: None,
            clock: None,
            dry_run: false,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6> ApplicationBuilder<(), T1, T2, T3, T4, T5, T6> {
    #[must_use]
    pub fn terminal(
        self,
        terminal: Terminal,
    ) -> ApplicationBuilder<Terminal, T1, T2, T3, T4, T5, T6> {
        ApplicationBuilder {
            terminal,
            feed_api: self.feed_api,
            feed_api_session: self.feed_api_session,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            github_client: self.github_client,
            clock: self.clock,
            dry_run: self.dry_run,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6> ApplicationBuilder<T1, (), T2, T3, T4, T5, T6> {
    #[must_use]
    pub fn client(
        self,
        client: synd_client::Client,
    ) -> ApplicationBuilder<T1, FeedApiRef, T2, T3, T4, T5, T6> {
        let feed_backend = FeedBackend::from_client(client, self.feed_api_session);
        self.feed_backend(feed_backend)
    }

    #[must_use]
    pub fn feed_backend(
        self,
        feed_backend: FeedBackend,
    ) -> ApplicationBuilder<T1, FeedApiRef, T2, T3, T4, T5, T6> {
        let (feed_api, feed_api_session) = feed_backend.into_parts();
        ApplicationBuilder {
            terminal: self.terminal,
            feed_api,
            feed_api_session,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            github_client: self.github_client,
            clock: self.clock,
            dry_run: self.dry_run,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6> ApplicationBuilder<T1, T2, (), T3, T4, T5, T6> {
    #[must_use]
    pub fn categories(
        self,
        categories: Categories,
    ) -> ApplicationBuilder<T1, T2, Categories, T3, T4, T5, T6> {
        ApplicationBuilder {
            terminal: self.terminal,
            feed_api: self.feed_api,
            feed_api_session: self.feed_api_session,
            categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            github_client: self.github_client,
            clock: self.clock,
            dry_run: self.dry_run,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6> ApplicationBuilder<T1, T2, T3, (), T4, T5, T6> {
    #[must_use]
    pub fn cache(self, cache: Cache) -> ApplicationBuilder<T1, T2, T3, Cache, T4, T5, T6> {
        ApplicationBuilder {
            terminal: self.terminal,
            feed_api: self.feed_api,
            feed_api_session: self.feed_api_session,
            categories: self.categories,
            cache,
            config: self.config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            github_client: self.github_client,
            clock: self.clock,
            dry_run: self.dry_run,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6> ApplicationBuilder<T1, T2, T3, T4, (), T5, T6> {
    #[must_use]
    pub fn config(self, config: Config) -> ApplicationBuilder<T1, T2, T3, T4, Config, T5, T6> {
        ApplicationBuilder {
            terminal: self.terminal,
            feed_api: self.feed_api,
            feed_api_session: self.feed_api_session,
            categories: self.categories,
            cache: self.cache,
            config,
            theme: self.theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            github_client: self.github_client,
            clock: self.clock,
            dry_run: self.dry_run,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6> ApplicationBuilder<T1, T2, T3, T4, T5, (), T6> {
    #[must_use]
    pub fn theme(self, theme: Theme) -> ApplicationBuilder<T1, T2, T3, T4, T5, Theme, T6> {
        ApplicationBuilder {
            terminal: self.terminal,
            feed_api: self.feed_api,
            feed_api_session: self.feed_api_session,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme,
            interactor: self.interactor,
            authenticator: self.authenticator,
            github_client: self.github_client,
            clock: self.clock,
            dry_run: self.dry_run,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6> ApplicationBuilder<T1, T2, T3, T4, T5, T6, ()> {
    #[must_use]
    pub fn interactor(
        self,
        interactor: Box<dyn Interact>,
    ) -> ApplicationBuilder<T1, T2, T3, T4, T5, T6, Box<dyn Interact>> {
        ApplicationBuilder {
            terminal: self.terminal,
            feed_api: self.feed_api,
            feed_api_session: self.feed_api_session,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            interactor,
            authenticator: self.authenticator,
            github_client: self.github_client,
            clock: self.clock,
            dry_run: self.dry_run,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6, T7> ApplicationBuilder<T1, T2, T3, T4, T5, T6, T7> {
    #[must_use]
    pub fn authenticator(self, authenticator: Authenticator) -> Self {
        Self {
            authenticator: Some(authenticator),
            ..self
        }
    }

    #[must_use]
    pub fn github_client(self, github_client: GithubClient) -> Self {
        Self {
            github_client: Some(github_client),
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
    pub fn dry_run(self, dry_run: bool) -> Self {
        Self { dry_run, ..self }
    }
}

impl ApplicationBuilder<Terminal, FeedApiRef, Categories, Cache, Config, Theme, Box<dyn Interact>> {
    #[must_use]
    pub fn build(self) -> Application<TermInit, SessPending> {
        Application::new(self)
    }
}
