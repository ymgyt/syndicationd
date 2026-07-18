use std::{ops::ControlFlow, time::Duration};

use crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind};
use futures_util::{Stream, StreamExt};
use ratatui::widgets::Widget;
use tracing::{debug, info, warn};

#[cfg(feature = "integration")]
use crate::auth::CredentialError;
use crate::{
    auth::{Credential, Verified},
    config::Categories,
    event::Event,
    interact::Interact,
    operation::Operation,
    terminal::{Terminal, TerminalGuard},
    ui::{
        self,
        theme::Theme,
        widgets::{gh_notifications::GitHubNotificationsWidget, root::AppWidget},
    },
};

mod direction;
pub(crate) use direction::{Direction, IndexOutOfRange};

mod in_flight;
pub(crate) use in_flight::{InFlight, RequestId, RequestSequence};

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
pub use outbound::feed::{ClientFeedApi, FeedApi, FeedApiRef};

mod app_config;
pub use app_config::{Config, Features};

mod state;
pub(crate) use state::TerminalFocus;

mod auth_flow;
mod commands;
pub(crate) mod component;
mod drivers;
mod events;
mod feeds;
mod idle;
mod integration;
mod operations;
use component::AppComponent;
use drivers::{DriverParts, Drivers};

const FEED_VIEW_SYNC_INTERVAL: Duration = Duration::from_mins(5);
const TIMELINE_INVALIDATION_DEBOUNCE: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Populate {
    Append,
    Replace,
}

/// Composition root that owns the event loop and connects components to drivers.
pub struct Application {
    drivers: Drivers,
    components: AppComponent,
    keymap: crate::keymap::Keymap,
    config: Config,
}

impl Application {
    /// Construct `ApplicationBuilder`
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder::default()
    }

    /// Construct `Application` from builder.
    /// Configure keymaps for terminal use
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
            github_client,
            categories,
            cache,
            config,
            theme,
            authenticator,
            interactor,
            clock,
            dry_run,
        } = builder;

        let components = AppComponent::new(&config.features, theme, categories, dry_run);
        let drivers = Drivers::new(DriverParts {
            terminal,
            feed_api,
            github_client,
            cache,
            authenticator,
            interactor,
            clock,
            throbber_timer_interval: config.throbber_timer_interval,
            idle_timer_interval: config.idle_timer_interval,
        });

        Self {
            drivers,
            components,
            keymap: crate::keymap::Keymap::new(config.keymaps.clone()),
            config,
        }
    }

    pub async fn run<S>(mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        let _guard = self.init_terminal()?;
        self.restore_github_notification_filter_options();
        self.start_session();
        self.event_loop(input).await;
        self.shutdown();
        Ok(())
    }

    /// Enter the terminal UI screen. The returned guard restores the terminal
    /// when dropped.
    /// The startup probe(dry run) tolerates init failure because test
    /// environments may not have a tty.
    fn init_terminal(&mut self) -> anyhow::Result<Option<TerminalGuard>> {
        match self.drivers.terminal.init() {
            Ok(guard) => Ok(Some(guard)),
            Err(err) => {
                if self.components.shell.should_quit() {
                    warn!("Failed to init terminal: {err}");
                    Ok(None)
                } else {
                    Err(err.into())
                }
            }
        }
    }

    /// Begin the feed api session: fire the initial fetches and mark the
    /// session authenticated.
    /// Also used by integration tests to start the application without
    /// initializing the terminal.
    pub fn start_session(&mut self) {
        self.initial_fetch();
        self.components.shell.auth.authenticated();
        self.reset_idle_timer();
    }

    fn shutdown(&mut self) {
        self.drivers.shutdown();

        if self.config.features.enable_github_notification {
            let options = self.components.github.notifications.filter_options();
            if let Err(err) = self
                .drivers
                .cache
                .persist_gh_notification_filter_options(options)
            {
                warn!("Failed to persist github notification filter options: {err}");
            }
        }
    }

    fn restore_github_notification_filter_options(&mut self) {
        if !self.config.features.enable_github_notification {
            return;
        }

        match self.drivers.cache.load_gh_notification_filter_options()
        {
            Ok(options) => {
                self.components.github.notifications =
                    GitHubNotificationsWidget::with_filter_options(options);
            }
            Err(err) => {
                warn!("Load github notification filter options: {err}");
            }
        }
    }

    #[cfg(feature = "integration")]
    async fn restore_credential(&self) -> Result<Verified<Credential>, CredentialError> {
        self.drivers.restore_credential().await
    }

    fn handle_restored_credential(&mut self, cred: Verified<Credential>) {
        self.set_credential(cred);
        self.start_session();
    }

    fn set_credential(&mut self, cred: Verified<Credential>) {
        self.drivers.set_credential(cred);
    }

    fn initial_fetch(&mut self) {
        info!("Initial fetch");
        self.perform_operation(Operation::StartFeedEventSubscription);
        self.perform_operation(Operation::FetchInitialFeedView {
            subscriptions_first: self.config.feeds_per_pagination,
            timeline_first: self.next_entries_first(0),
        });
        if self.config.features.enable_github_notification
            && let Some(operation) = self.components.github.fetch_next_notifications_if_needed()
        {
            self.perform_operation(operation);
        }
        self.perform_operation(Operation::ScheduleFeedViewSync);
    }

    async fn event_loop<S>(&mut self, input: &mut S)
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.render();

        loop {
            if self.event_loop_until_idle(input).await.is_break() {
                break;
            }
        }
    }

    pub async fn event_loop_until_idle<S>(&mut self, input: &mut S) -> ControlFlow<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        enum PollResult {
            Terminal(std::io::Result<CrosstermEvent>),
            Job(anyhow::Result<Event>),
            BackgroundJob(anyhow::Result<Event>),
            FeedEvent(drivers::FeedEventMessage),
            Throbber,
            Idle,
        }

        loop {
            let result = {
                let pollers = self.drivers.pollers();
                tokio::select! {
                    biased;

                    Some(event) = input.next() => PollResult::Terminal(event),
                    Some(event) = pollers.jobs.next() => PollResult::Job(event),
                    Some(event) = pollers.background_jobs.next() => PollResult::BackgroundJob(event),
                    Some(event) = pollers.feed_events.recv() => PollResult::FeedEvent(event),
                    ()  = pollers.in_flight.throbber_timer() => PollResult::Throbber,
                    () = &mut *pollers.idle_timer => PollResult::Idle,
                }
            };

            let keymap_layers = self.active_keymap_layers();
            match result {
                PollResult::Terminal(event) => self.handle_terminal_event(event),
                PollResult::Job(event) | PollResult::BackgroundJob(event) => {
                    self.apply_job_result(event);
                }
                PollResult::FeedEvent(message) => match message {
                    drivers::FeedEventMessage::Event(event) => {
                        self.apply_event(Event::RegistryFeed { event });
                    }
                    drivers::FeedEventMessage::Interrupted => {
                        self.apply_event(Event::FeedEventSubscriptionInterrupted);
                    }
                },
                PollResult::Throbber => {
                    self.apply_event(Event::RenderThrobber);
                }
                PollResult::Idle => {
                    self.apply_event(Event::Idle);
                }
            }

            // Discard a pending key sequence when the keymap context changed
            if self.active_keymap_layers() != keymap_layers {
                self.keymap.clear_pending();
            }

            if self.components.shell.should_quit() {
                self.components.shell.clear_quit_request(); // for testing
                break ControlFlow::Break(());
            }

            self.render();
        }
    }

    fn apply_job_result(&mut self, result: anyhow::Result<Event>) {
        match result {
            Ok(event) => self.apply_event(event),
            Err(err) => self.apply_event(Event::Error {
                message: err.to_string(),
            }),
        }
    }

    fn render(&mut self) {
        let components = &self.components;
        self.drivers
            .render(|frame, in_flight, now| {
                let cx = ui::Context {
                    theme: &components.shell.theme,
                    in_flight,
                    categories: &components.shell.categories,
                    focus: components.shell.focus(),
                    now,
                    tab: components.shell.tabs.current(),
                };
                let root = AppWidget::new(components, cx);
                Widget::render(root, frame.area(), frame.buffer_mut());
            })
            .expect("Failed to render");
    }

    fn handle_terminal_event(&mut self, event: std::io::Result<CrosstermEvent>) {
        let event = match event {
            Ok(event) => event,
            Err(err) => {
                self.apply_event(Event::Error {
                    message: format!("read terminal event failed: {err}"),
                });
                return;
            }
        };

        match event {
            CrosstermEvent::Resize(..) => self.apply_event(Event::TerminalResized),
            CrosstermEvent::FocusGained => {
                self.components.shell.focus_gained();
            }
            CrosstermEvent::FocusLost => {
                self.components.shell.focus_lost();
            }
            CrosstermEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            }) => {}
            CrosstermEvent::Key(key) => {
                debug!("Handle key event: {key:?}");

                // An error message stays visible until the next key input
                self.components.shell.prompt.clear_error_message();
                self.reset_idle_timer();

                self.handle_keymap(key);
            }
            _ => {}
        }
    }
}
