use std::sync::Arc;
use tracing::warn;
use tuiframework::{Application as Tui, RunError};

use crate::{config::Categories, interact::Interact, ui::theme::Theme};

mod direction;
mod effects;

pub(crate) use direction::Direction;

mod request;
pub(crate) use request::{Generation, RequestError, RequestId, RequestKind};

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
use compositor::SyndUi;
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
    tui: Tui<SyndUi, Drivers>,
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
            Box<dyn Interact>,
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
            interactor,
            clock,
            html_converter,
            authentication_required,
        } = builder;
        let period = config.throbber_timer_interval;
        let clock: Arc<dyn Clock> = Arc::from(clock.unwrap_or_else(|| Box::new(SystemClock)));
        let (mut ui, keymap) = SyndUi::new(
            config,
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
            interactor,
            clock,
            html_converter,
        });

        if ui.gh.enabled {
            match drivers.cache.load_gh_notification_filter_options() {
                Ok(options) => ui.gh.restore_filter_options(options),
                Err(error) => warn!("Load GitHub notification filter options: {error}"),
            }
        }
        let tui = Tui::builder()
            .components(ui)
            .driver(drivers)
            .keymap(keymap)
            .tick_interval(period)
            .build();
        Self { tui }
    }
    pub async fn run(mut self) -> anyhow::Result<()> {
        let result = self.tui.run().await;
        if !matches!(result, Err(RunError::Setup(_))) {
            self.persist_gh_filter();
        }
        Ok(result?)
    }
    fn persist_gh_filter(&self) {
        if !self.tui.components().gh.enabled {
            return;
        }
        let options = self.tui.components().gh.filter_options_snapshot();
        if let Err(error) = self
            .tui
            .driver()
            .cache
            .persist_gh_notification_filter_options(options)
        {
            warn!("Failed to persist GitHub notification filter options: {error}");
        }
    }
}
