use crossterm::event::Event as CrosstermEvent;
use futures_util::{Stream, StreamExt as _};
use std::{ops::ControlFlow, pin::Pin, time::Duration};
use tokio::time::{self, Instant, Sleep};
use tracing::warn;

use crate::{
    config::Categories,
    interact::Interact,
    operation::Operations,
    terminal::{Terminal, TerminalGuard},
    ui::theme::Theme,
};

mod direction;
pub(crate) use direction::Direction;

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

mod state;
pub(crate) use state::TerminalFocus;

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
    drivers: Drivers,
    ui: Compositor,
    throbber: Pin<Box<Sleep>>,
    throbber_interval: Duration,
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
        let throbber_interval = config.throbber_timer_interval;
        let ui = Compositor::new(config, theme, categories, authentication);
        let drivers = Drivers::new(DriverParts {
            terminal,
            feed_api,
            gh_client,
            cache,
            authenticator,
            interactor,
            clock,
        });

        Self {
            drivers,
            ui,
            throbber: Box::pin(time::sleep(throbber_interval)),
            throbber_interval,
        }
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.drivers.render(|frame, now| self.ui.render(frame, now))
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
            self.ui.components.shell.authentication(),
            AuthenticationState::NotRequired
        ) {
            let operations = self.ui.bootstrap();
            self.drivers.dispatch(operations);
        }
        Ok(terminal)
    }

    async fn event_loop<S>(&mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.render()?;
        while self.process_next(input).await?.is_continue() {}
        Ok(())
    }

    async fn process_next<S>(&mut self, input: &mut S) -> anyhow::Result<ControlFlow<()>>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        let busy = self.ui.is_busy();
        let operations = tokio::select! {
            biased;
            event = input.next() => match event {
                Some(event) => self.ui.apply_terminal_event(&event?),
                None => return Ok(ControlFlow::Break(())),
            },
            Some(event) = self.drivers.next() => self.ui.apply_event(event),
            () = self.throbber.as_mut(), if busy => {
                self.ui.components.shell.in_flight.tick();
                self.throbber.as_mut().reset(Instant::now() + self.throbber_interval);
                Operations::Nop
            }
        };
        if !busy && self.ui.is_busy() {
            self.throbber
                .as_mut()
                .reset(Instant::now() + self.throbber_interval);
        }
        self.drivers.dispatch(operations);
        if self.ui.components.shell.should_quit() {
            return Ok(ControlFlow::Break(()));
        }
        self.render()?;
        Ok(ControlFlow::Continue(()))
    }

    fn restore_persisted_state(&mut self) {
        if !self.ui.features.enable_gh_notification {
            return;
        }
        match self.drivers.cache.load_gh_notification_filter_options() {
            Ok(options) => self.ui.components.gh.restore_filter_options(options),
            Err(error) => warn!("Load GitHub notification filter options: {error}"),
        }
    }

    fn shutdown(&mut self) {
        self.drivers.shutdown();
        self.persist_state();
    }

    fn persist_state(&self) {
        if !self.ui.features.enable_gh_notification {
            return;
        }
        let options = self.ui.components.gh.filter_options_snapshot();
        if let Err(error) = self
            .drivers
            .cache
            .persist_gh_notification_filter_options(options)
        {
            warn!("Failed to persist GitHub notification filter options: {error}");
        }
    }
}
