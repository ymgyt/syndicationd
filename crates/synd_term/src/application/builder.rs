use crate::{
    application::{Application, Authenticator, Cache, Config},
    client::Client,
    config::Categories,
    interact::Interactor,
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
> {
    pub(super) terminal: Terminal,
    pub(super) client: Client,
    pub(super) categories: Categories,
    pub(super) cache: Cache,
    pub(super) config: Config,
    pub(super) theme: Theme,

    pub(super) authenticator: Option<Authenticator>,
    pub(super) interactor: Option<Interactor>,
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
            authenticator: None,
            interactor: None,
        }
    }
}

impl<T1, T2, T3, T4, T5> ApplicationBuilder<(), T1, T2, T3, T4, T5> {
    #[must_use]
    pub fn terminal(self, terminal: Terminal) -> ApplicationBuilder<Terminal, T1, T2, T3, T4, T5> {
        ApplicationBuilder {
            terminal,
            client: self.client,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            authenticator: self.authenticator,
            interactor: self.interactor,
        }
    }
}

impl<T1, T2, T3, T4, T5> ApplicationBuilder<T1, (), T2, T3, T4, T5> {
    #[must_use]
    pub fn client(self, client: Client) -> ApplicationBuilder<T1, Client, T2, T3, T4, T5> {
        ApplicationBuilder {
            terminal: self.terminal,
            client,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            authenticator: self.authenticator,
            interactor: self.interactor,
        }
    }
}

impl<T1, T2, T3, T4, T5> ApplicationBuilder<T1, T2, (), T3, T4, T5> {
    #[must_use]
    pub fn categories(
        self,
        categories: Categories,
    ) -> ApplicationBuilder<T1, T2, Categories, T3, T4, T5> {
        ApplicationBuilder {
            terminal: self.terminal,
            client: self.client,
            categories,
            cache: self.cache,
            config: self.config,
            theme: self.theme,
            authenticator: self.authenticator,
            interactor: self.interactor,
        }
    }
}

impl<T1, T2, T3, T4, T5> ApplicationBuilder<T1, T2, T3, (), T4, T5> {
    #[must_use]
    pub fn cache(self, cache: Cache) -> ApplicationBuilder<T1, T2, T3, Cache, T4, T5> {
        ApplicationBuilder {
            terminal: self.terminal,
            client: self.client,
            categories: self.categories,
            cache,
            config: self.config,
            theme: self.theme,
            authenticator: self.authenticator,
            interactor: self.interactor,
        }
    }
}

impl<T1, T2, T3, T4, T5> ApplicationBuilder<T1, T2, T3, T4, (), T5> {
    #[must_use]
    pub fn config(self, config: Config) -> ApplicationBuilder<T1, T2, T3, T4, Config, T5> {
        ApplicationBuilder {
            terminal: self.terminal,
            client: self.client,
            categories: self.categories,
            cache: self.cache,
            config,
            theme: self.theme,
            authenticator: self.authenticator,
            interactor: self.interactor,
        }
    }
}

impl<T1, T2, T3, T4, T5> ApplicationBuilder<T1, T2, T3, T4, T5, ()> {
    #[must_use]
    pub fn theme(self, theme: Theme) -> ApplicationBuilder<T1, T2, T3, T4, T5, Theme> {
        ApplicationBuilder {
            terminal: self.terminal,
            client: self.client,
            categories: self.categories,
            cache: self.cache,
            config: self.config,
            theme,
            authenticator: self.authenticator,
            interactor: self.interactor,
        }
    }
}

impl<T1, T2, T3, T4, T5, T6> ApplicationBuilder<T1, T2, T3, T4, T5, T6> {
    #[must_use]
    pub fn authenticator(self, authenticator: Authenticator) -> Self {
        Self {
            authenticator: Some(authenticator),
            ..self
        }
    }

    #[must_use]
    pub fn interactor(self, interactor: Interactor) -> Self {
        Self {
            interactor: Some(interactor),
            ..self
        }
    }
}

impl ApplicationBuilder<Terminal, Client, Categories, Cache, Config, Theme> {
    #[must_use]
    pub fn build(self) -> Application {
        Application::new(self)
    }
}
