use std::{ops::ControlFlow, time::Duration};

use crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind};
use futures_util::{Stream, StreamExt};
use ratatui::widgets::Widget;
use synd_feed::types::FeedUrl;
use tracing::{debug, info, warn};

use crate::{
    auth::{Credential, CredentialError, Verified},
    config::Categories,
    event::Event,
    interact::Interact,
    operation::Operation,
    terminal::Terminal,
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
pub use outbound::feed::{FeedApi, FeedApiRef, FeedApiSession, FeedBackend};

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
mod lifecycle;
mod operations;
use component::AppComponent;
use drivers::{DriverParts, Drivers};
use lifecycle::Lifecycle;
pub use lifecycle::{SessPending, SessReady, TermReady, TermRestored, TermUninit};

const FEED_REFRESH_POLL_ATTEMPTS: u16 = 300;
const FEED_REFRESH_POLL_INTERVAL: Duration = Duration::from_secs(1);
const FEED_VIEW_SYNC_INTERVAL: Duration = Duration::from_mins(5);
const TIMELINE_INVALIDATION_DEBOUNCE: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Populate {
    Append,
    Replace,
}

/// Composition root that owns the event loop and connects components to drivers.
pub struct Application<Term = TermUninit, Sess = SessPending> {
    drivers: Drivers,
    components: AppComponent,
    keymap: crate::keymap::Keymap,
    config: Config,
    _lifecycle: Lifecycle<Term, Sess>,
}

pub enum StartSession {
    Ready(Application<TermReady, SessReady>),
    Pending(Application<TermReady, SessPending>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct FeedRefreshPollKey {
    url: FeedUrl,
    request_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TimelineInvalidationState {
    Clean,
    DirtyWaiting,
    Refetching,
    DirtyWhileRefetching,
}

impl FeedRefreshPollKey {
    fn new(url: FeedUrl, request_id: String) -> Self {
        Self { url, request_id }
    }
}

impl Application<TermUninit, SessPending> {
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
            feed_api_session,
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
            feed_api_session,
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
            _lifecycle: Lifecycle::new(),
        }
    }

    #[expect(clippy::large_futures)]
    pub async fn run<S>(self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        match self.init().await? {
            StartSession::Ready(app) => {
                let _app = app.run_until_exit(input).await?;
            }
            StartSession::Pending(app) => {
                let _app = app.run_until_exit(input).await?;
            }
        }

        Ok(())
    }

    /// Initialize terminal and application state.
    async fn init(self) -> anyhow::Result<StartSession> {
        let mut app = self.init_terminal()?;
        app.restore_github_notification_filter_options();
        Ok(app.start_session().await)
    }
}

impl<Sess> Application<TermUninit, Sess> {
    pub fn init_terminal(mut self) -> anyhow::Result<Application<TermReady, Sess>> {
        match self.drivers.handles.terminal.init() {
            Ok(()) => Ok(self.into_terminal_ready()),
            Err(err) => {
                if self.components.shell.should_quit() {
                    warn!("Failed to init terminal: {err}");
                    Ok(self.into_terminal_ready())
                } else {
                    Err(err.into())
                }
            }
        }
    }

    #[cfg(feature = "integration")]
    #[must_use]
    pub fn assume_terminal_ready(self) -> Application<TermReady, Sess> {
        self.into_terminal_ready()
    }

    fn into_terminal_ready(self) -> Application<TermReady, Sess> {
        let Self {
            drivers,
            components,
            keymap,
            config,
            _lifecycle: _,
        } = self;

        Application {
            drivers,
            components,
            keymap,
            config,
            _lifecycle: Lifecycle::new(),
        }
    }
}

impl Application<TermReady, SessPending> {
    pub async fn start_session(mut self) -> StartSession {
        if self.start_feed_session_in_place().await {
            StartSession::Ready(self.into_session_ready())
        } else {
            StartSession::Pending(self)
        }
    }

    fn into_session_ready(self) -> Application<TermReady, SessReady> {
        let Self {
            drivers,
            components,
            keymap,
            config,
            _lifecycle: _,
        } = self;

        Application {
            drivers,
            components,
            keymap,
            config,
            _lifecycle: Lifecycle::new(),
        }
    }
}

impl<Sess> Application<TermReady, Sess> {
    async fn run_until_exit<S>(
        mut self,
        input: &mut S,
    ) -> anyhow::Result<Application<TermRestored, Sess>>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.event_loop(input).await;
        self.stop()
    }

    pub fn stop(mut self) -> anyhow::Result<Application<TermRestored, Sess>> {
        self.shutdown_drivers();
        self.cleanup()?;

        let Self {
            drivers,
            components,
            keymap,
            config,
            _lifecycle: _,
        } = self;

        Ok(Application {
            drivers,
            components,
            keymap,
            config,
            _lifecycle: Lifecycle::new(),
        })
    }
}

impl<Term, Sess> Application<Term, Sess> {
    fn restore_github_notification_filter_options(&mut self) {
        if !self.config.features.enable_github_notification {
            return;
        }

        match self.drivers.load_gh_notification_filter_options() {
            Ok(options) => {
                self.components.github.notifications =
                    GitHubNotificationsWidget::with_filter_options(options);
            }
            Err(err) => {
                warn!("Load github notification filter options: {err}");
            }
        }
    }

    async fn start_feed_session_in_place(&mut self) -> bool {
        if self.drivers.feed_api_session_requires_user_credential() {
            match self.restore_credential().await {
                Ok(cred) => {
                    self.handle_restored_credential(cred);
                    true
                }
                Err(err) => {
                    warn!("Restore credential: {err}");
                    false
                }
            }
        } else {
            self.enter_feed_api_session();
            true
        }
    }

    async fn restore_credential(&self) -> Result<Verified<Credential>, CredentialError> {
        self.drivers.restore_credential().await
    }

    fn handle_restored_credential(&mut self, cred: Verified<Credential>) {
        self.set_credential(cred);
        self.enter_feed_api_session();
    }

    fn enter_feed_api_session(&mut self) {
        self.initial_fetch();
        self.components.shell.auth.authenticated();
        self.reset_idle_timer();
        self.should_render();
        self.keymap.clear_pending();
    }

    fn set_credential(&mut self, cred: Verified<Credential>) {
        self.drivers.set_credential(cred);
    }

    fn initial_fetch(&mut self) {
        info!("Initial fetch");
        if self.drivers.supports_feed_event_subscription() {
            self.perform_operation(Operation::StartFeedEventSubscription);
        }
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

    /// Restore terminal state and print something to console if necessary
    fn cleanup(&mut self) -> anyhow::Result<()> {
        if self.config.features.enable_github_notification {
            let options = self.components.github.notifications.filter_options();
            match self.drivers.persist_gh_notification_filter_options(options) {
                Ok(()) => {}
                Err(err) => {
                    warn!("Failed to persist github notification filter options: {err}");
                }
            }
        }

        self.drivers.handles.terminal.restore()?;

        Ok(())
    }

    fn shutdown_drivers(&mut self) {
        self.drivers.shutdown();
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

            if self.components.shell.should_render() {
                self.render();
                self.components.shell.clear_render_request();
                self.components.shell.prompt.clear_error_message();
            }

            if self.components.shell.should_quit() {
                self.components.shell.clear_quit_request(); // for testing
                break ControlFlow::Break(());
            }
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
            CrosstermEvent::Resize(columns, rows) => self.apply_event(Event::TerminalResized {
                _columns: columns,
                _rows: rows,
            }),
            CrosstermEvent::FocusGained => {
                self.should_render();
                if let Some(command) = self.components.shell.focus_gained() {
                    self.apply_command(command);
                }
            }
            CrosstermEvent::FocusLost => {
                self.should_render();
                if let Some(command) = self.components.shell.focus_lost() {
                    self.apply_command(command);
                }
            }
            CrosstermEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            }) => {}
            CrosstermEvent::Key(key) => {
                debug!("Handle key event: {key:?}");

                self.reset_idle_timer();

                self.handle_keymap(key);
            }
            _ => {}
        }
    }
}
