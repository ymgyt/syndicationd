use crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind};
use futures_util::{Stream, StreamExt as _};
use ratatui::widgets::Widget;
use tracing::{debug, warn};

use crate::{
    config::Categories,
    event::Event,
    interact::Interact,
    operation::Operations,
    terminal::{Terminal, TerminalGuard},
    ui::{self, theme::Theme, widgets::root::AppWidget},
};

mod direction;
pub(crate) use direction::Direction;

mod request;
pub(crate) use request::{RequestError, RequestId, RequestKind};

mod in_flight;
pub(crate) use in_flight::{InFlightRequests, InFlightStatus, RequestProgress};

mod input_parser;
mod keymap;

pub use crate::auth::authenticator::{Authenticator, DeviceFlows, JwtService};

mod clock;
pub use clock::{Clock, SystemClock};

mod cache;
pub use cache::{Cache, LoadCacheError, PersistCacheError};

mod builder;
pub use builder::ApplicationBuilder;

pub mod outbound;
pub use outbound::feed::{ClientFeedApi, FeedApi, FeedApiRef, FeedEventWatch};

mod app_config;
pub use app_config::{Config, Features};

mod state;
pub(crate) use state::TerminalFocus;

mod commands;
pub(crate) mod component;
mod drivers;
mod events;
mod integration;

use component::{AuthenticationState, Components};
use drivers::{DriverParts, Drivers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Populate {
    Append,
    Replace,
}

/// Composition root that owns orchestration between state and external drivers.
pub struct Application {
    drivers: Drivers,
    components: Components,
    keymap: crate::keymap::Keymap,
    config: Config,
}

impl Application {
    /// Construct `ApplicationBuilder`.
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder::default()
    }

    fn new(
        builder: ApplicationBuilder<
            Terminal,
            FeedApiRef,
            Categories,
            Cache,
            Config,
            Theme,
            Box<dyn Interact>,
        >,
    ) -> Self {
        let ApplicationBuilder {
            terminal,
            feed_api,
            gh_client,
            categories,
            cache,
            config,
            theme,
            authenticator,
            interactor,
            clock,
            authentication_required,
        } = builder;
        let components = {
            let authentication = if authentication_required {
                AuthenticationState::Required
            } else {
                AuthenticationState::NotRequired
            };
            Components::new(&config.features, theme, categories, authentication)
        };
        let drivers = {
            let parts = DriverParts {
                terminal,
                feed_api,
                gh_client,
                cache,
                authenticator,
                interactor,
                clock,
                throbber_timer_interval: config.throbber_timer_interval,
                idle_timer_interval: config.idle_timer_interval,
            };
            Drivers::new(parts)
        };

        Self {
            drivers,
            components,
            keymap: crate::keymap::Keymap::new(config.keymaps.clone()),
            config,
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        let components = &self.components;
        self.drivers.render(|frame, now| {
            let cx = ui::Context {
                theme: &components.shell.theme,
                in_flight: &components.shell.in_flight,
                categories: &components.shell.categories,
                focus: components.shell.focus(),
                now,
                tab: components.shell.tabs.current(),
            };
            let root = AppWidget::new(components, cx);
            Widget::render(root, frame.area(), frame.buffer_mut());
        })
    }

    pub(super) fn reset_idle_timer(&mut self) {
        self.drivers
            .reset_idle_timer(self.config.idle_timer_interval);
    }

    pub async fn run<S>(mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        let _terminal = self.initialize()?;
        let result = self.event_loop(input).await;
        self.shutdown();
        result
    }

    fn initialize(&mut self) -> anyhow::Result<TerminalGuard> {
        let terminal = self.drivers.terminal.init()?;
        self.restore_persisted_state();
        if matches!(
            self.components.shell.authentication(),
            AuthenticationState::NotRequired
        ) {
            let operations = self.bootstrap().into();
            self.drivers.dispatch(operations);
            self.reset_idle_timer();
        }
        Ok(terminal)
    }

    /// Starts the first remote synchronization after API access becomes available.
    pub(super) fn bootstrap(&mut self) -> impl Into<Operations> {
        let feeds: Operations = self
            .components
            .feeds
            .bootstrap(self.config.feeds_per_pagination, self.config.entries_limit)
            .into();
        let gh: Operations = if self.config.features.enable_gh_notification {
            self.components.gh.bootstrap().into()
        } else {
            Operations::Nop
        };
        [feeds, gh]
    }

    async fn event_loop<S>(&mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.render()?;
        self.process_until_quit(input).await
    }

    pub(super) async fn process_until_quit<S>(&mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        loop {
            self.process_next(input).await?;
            if self.components.shell.take_should_quit() {
                break;
            }
            self.render()?;
        }
        Ok(())
    }

    async fn process_next<S>(&mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        let operations = tokio::select! {
            biased;

            Some(event) = input.next() => self.apply_terminal_event(&event?),
            Some(event) = self.drivers.next() => self.apply_event(event),
        };
        self.drivers.dispatch(operations);

        let layers = self.active_keymap_layers();
        self.keymap.sync_layers(&layers);
        Ok(())
    }

    fn apply_terminal_event(&mut self, event: &CrosstermEvent) -> Operations {
        match event {
            CrosstermEvent::Resize(..) => self.apply_event(Event::TerminalResized),
            CrosstermEvent::FocusGained => self.apply_event(Event::TerminalFocusGained),
            CrosstermEvent::FocusLost => self.apply_event(Event::TerminalFocusLost),
            CrosstermEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            }) => Operations::Nop,
            CrosstermEvent::Key(key) => {
                debug!("Handle key event: {key:?}");
                self.components.shell.prompt.clear_error_message();
                self.reset_idle_timer();
                self.resolve_keymap(*key)
                    .map_or(Operations::Nop, |command| self.apply_command(command))
            }
            _ => Operations::Nop,
        }
    }

    fn restore_persisted_state(&mut self) {
        if !self.config.features.enable_gh_notification {
            return;
        }
        match self.drivers.cache.load_gh_notification_filter_options() {
            Ok(options) => self.components.gh.restore_filter_options(options),
            Err(error) => warn!("Load GitHub notification filter options: {error}"),
        }
    }

    fn shutdown(&mut self) {
        self.drivers.shutdown();
        self.persist_state();
    }

    fn persist_state(&self) {
        if !self.config.features.enable_gh_notification {
            return;
        }
        let options = self.components.gh.filter_options_snapshot();
        if let Err(error) = self
            .drivers
            .cache
            .persist_gh_notification_filter_options(options)
        {
            warn!("Failed to persist GitHub notification filter options: {error}");
        }
    }
}
