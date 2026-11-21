use crossterm::event::Event as CrosstermEvent;
use futures_util::{Stream, StreamExt as _};
use tracing::warn;

use crate::{
    config::Categories, event::Event, interact::Interact, terminal::Terminal, ui::theme::Theme,
};

mod direction;
mod effects;
mod throbber;
pub(crate) use direction::Direction;
use throbber::Throbber;

mod request;
pub(crate) use request::{RequestError, RequestId, RequestKind};

mod in_flight;
pub(crate) use in_flight::{InFlightRequests, InFlightStatus, RequestProgress};

mod input_parser;

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

pub(crate) use compositor::TerminalFocus;

pub(crate) mod component;
mod compositor;
mod overlay;
pub(crate) use overlay::ComponentId;
mod drivers;
#[cfg(feature = "integration")]
mod integration;
use component::AuthenticationState;
use compositor::Compositor;
use drivers::{DriverParts, Drivers};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Populate {
    Append,
    Replace,
}

/// Composition root that owns orchestration between state and external drivers.
pub struct Application {
    terminal: Terminal,
    drivers: Drivers,
    compositor: Compositor,
    throbber: Throbber,
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
        let authentication = if authentication_required {
            AuthenticationState::Required
        } else {
            AuthenticationState::NotRequired
        };
        let throbber = Throbber::new(config.throbber_timer_interval);
        let compositor = Compositor::new(config, theme, categories, authentication);
        let drivers = Drivers::new(DriverParts {
            feed_api,
            gh_client,
            cache,
            authenticator,
            interactor,
            clock,
        });

        Self {
            terminal,
            drivers,
            compositor,
            throbber,
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        let now = self.drivers.clock.now();
        self.terminal
            .render(|frame| self.compositor.render(frame, now))
    }

    pub async fn run<S>(mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        let _terminal = self.terminal.init()?;
        self.restore_gh_filter();
        if matches!(
            self.compositor.components.shell.authentication(),
            AuthenticationState::NotRequired
        ) {
            let operations = self.compositor.apply_event(&Event::ApiAccessReady);
            self.drivers.dispatch(operations, &mut self.terminal);
        }
        let result = self.event_loop(input).await;
        self.drivers.shutdown();
        self.persist_gh_filter();
        result
    }

    async fn event_loop<S>(&mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        while !self.compositor.should_quit() {
            self.render()?;
            self.process_next(input).await?;
        }
        Ok(())
    }

    async fn process_next<S>(&mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        tokio::select! {
            biased;
            event = input.next() => match event {
                Some(event) => {
                    let operations = self.compositor.apply_terminal_event(&event?);
                    self.drivers.dispatch(operations, &mut self.terminal);
                }
                None => self.compositor.quit(),
            },
            Some(event) = self.drivers.next() => {
                let operations = self.compositor.apply_event(&event);
                self.drivers.dispatch(operations, &mut self.terminal);
            }
            () = self.throbber.tick(), if self.compositor.is_busy() => self.compositor.tick(),
        }
        Ok(())
    }

    fn restore_gh_filter(&mut self) {
        if !self.compositor.components.gh.enabled {
            return;
        }
        match self.drivers.cache.load_gh_notification_filter_options() {
            Ok(options) => self
                .compositor
                .components
                .gh
                .restore_filter_options(options),
            Err(error) => warn!("Load GitHub notification filter options: {error}"),
        }
    }

    fn persist_gh_filter(&self) {
        if !self.compositor.components.gh.enabled {
            return;
        }
        let options = self.compositor.components.gh.filter_options_snapshot();
        if let Err(error) = self
            .drivers
            .cache
            .persist_gh_notification_filter_options(options)
        {
            warn!("Failed to persist GitHub notification filter options: {error}");
        }
    }
}
