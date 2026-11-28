use futures_util::Stream;
use std::{io, sync::Arc};
use tracing::{info, warn};
use tuiframework::crossterm::event::{Event as CrosstermEvent, EventStream};
use tuiframework::event_loop::Ready;
use tuiframework::{
    AnimationTimer, ApplicationEffects, EventLoop, Hooks, RenderTarget, Terminal, TerminalOptions,
};

use crate::{config::Categories, external_tools::ExternalTools, ui::theme::Theme};

mod direction;
mod effects;

pub(crate) use direction::Direction;

mod request;
pub(crate) use request::{Generation, RequestError, RequestId, RequestKind};

mod in_flight;
pub(crate) use in_flight::{InFlightRequests, InFlightStatus, RequestProgress};

pub use crate::auth::authenticator::{Authenticator, DeviceFlows, JwtService};

mod clock;
pub use clock::{Clock, SystemClock};

mod cache;
pub use cache::{Cache, LoadCacheError, PersistCacheError};

mod builder;
pub use builder::ApplicationBuilder;

pub mod outbound;
pub use outbound::feed::{FeedApi, FeedApiRef, FeedClient, FeedEventWatch};

mod app_config;
pub use app_config::{Config, Features};

pub(crate) mod component;
mod overlay;
#[cfg(test)]
mod tests;
pub(crate) use overlay::ComponentId;
mod drivers;
#[cfg(feature = "integration")]
mod integration;
use component::Screens;
use drivers::{DriverParts, Drivers};

#[cfg(feature = "integration")]
pub use integration::TestApplication;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Populate {
    Append,
    Replace,
}

/// Wires the synd screens and services into the terminal application.
pub struct Application {
    screens: Screens,
    keymap: crate::keymap::Keymap,
    drivers: Drivers,
    animation_timer: AnimationTimer,
}

impl Application {
    /// Construct `ApplicationBuilder`.
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder::default()
    }

    fn new(
        builder: ApplicationBuilder<
            FeedApiRef,
            Categories,
            Cache,
            Config,
            Theme,
            Box<dyn ExternalTools>,
        >,
    ) -> Self {
        let ApplicationBuilder {
            feed_api,
            gh_client,
            categories,
            cache,
            config,
            theme,
            authenticator,
            external_tools,
            clock,
            html_converter,
            authentication_required,
        } = builder;
        let clock: Arc<dyn Clock> = Arc::from(clock.unwrap_or_else(|| Box::new(SystemClock)));
        let ui = Screens::new(
            &config,
            theme,
            categories,
            authentication_required,
            Arc::clone(&clock),
        );
        let drivers = Drivers::new(DriverParts {
            feed_api,
            gh_client,
            cache,
            authenticator,
            external_tools,
            clock,
            html_converter,
        });

        Self {
            screens: ui,
            keymap: config.keymaps.0,
            drivers,
            animation_timer: AnimationTimer::new(config.throbber_timer_interval),
        }
    }

    fn event_loop<T, I>(
        self,
        terminal: T,
        input: I,
    ) -> EventLoop<Ready<'static, Screens, Drivers>, T, I>
    where
        T: RenderTarget,
        I: Stream<Item = io::Result<CrosstermEvent>> + Unpin,
    {
        let hooks = Hooks::default()
            .on_start(Box::new(Self::start))
            .on_exit(Box::new(Self::persist_gh_filter));
        EventLoop::builder()
            .components(self.screens)
            .keymap(self.keymap)
            .overlay(overlay::layout())
            .driver(self.drivers)
            .terminal(terminal)
            .input(input)
            .animation_timer(self.animation_timer)
            .hooks(hooks)
            .build()
    }

    pub async fn run(self) -> io::Result<()> {
        info!("Starting terminal UI");
        let options = TerminalOptions::default()
            .with_bracketed_paste(true)
            .with_restore_on_panic_abort(true);
        let terminal = Terminal::init(options)?;
        let event_loop = self.event_loop(terminal, EventStream::new());
        Box::pin(event_loop.run()).await.map_err(io::Error::from)
    }

    fn start(ui: &mut Screens, drivers: &mut Drivers, effects: &mut ApplicationEffects<Screens>) {
        if ui.gh.enabled {
            match drivers.cache.load_gh_notification_filter_options() {
                Ok(options) => ui.gh.restore_filter_options(options),
                Err(error) => warn!("Load GitHub notification filter options: {error}"),
            }
        }
        if ui.login.has_api_access() {
            effects.emit(crate::event::Event::ApiAccessReady);
        }
    }

    fn persist_gh_filter(ui: &Screens, drivers: &mut Drivers) {
        let gh = &ui.gh;
        if gh.enabled {
            let options = gh.notifications.filter_options();
            if let Err(error) = drivers
                .cache
                .persist_gh_notification_filter_options(options)
            {
                warn!("Failed to persist GitHub notification filter options: {error}");
            }
        }
    }
}
