use crate::{
    application::{Application, Authenticator, Cache, Clock, Config},
    client::{github::GithubClient, Client},
    config::Categories,
    interact::Interact,
    terminal::Terminal,
    ui::theme::Theme,
};

pub struct ApplicationBuilder<
    Terminal = (),
    Client = (),
    Categories = (),
    Cache = (),
    Config = (),
    Theme = (),
    Interactor = (),
> {
    pub(super) terminal: Terminal,
    pub(super) client: Client,
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
            client: (),
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
            client: self.client,
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
    pub fn client(self, client: Client) -> ApplicationBuilder<T1, Client, T2, T3, T4, T5, T6> {
        ApplicationBuilder {
            terminal: self.terminal,
            client,
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
            client: self.client,
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
            client: self.client,
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
            client: self.client,
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
            client: self.client,
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
            client: self.client,
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

impl ApplicationBuilder<Terminal, Client, Categories, Cache, Config, Theme, Box<dyn Interact>> {
    #[must_use]
    pub fn build(self) -> Application {
        Application::new(self)
    }
}
