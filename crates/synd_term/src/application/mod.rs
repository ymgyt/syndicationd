use std::{future, pin::Pin, time::Duration};

use crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind};
use futures_util::{FutureExt, Stream, StreamExt};
use ratatui::{style::palette::tailwind, widgets::Widget};
use synd_auth::device_flow::{
    self, DeviceAccessTokenResponse, DeviceAuthorizationResponse, DeviceFlow,
};
use tokio::time::{Instant, Sleep};

use crate::{
    auth::{AuthenticationProvider, Credential},
    client::{mutation::subscribe_feed::SubscribeFeedInput, Client},
    command::Command,
    config::{self, Categories},
    interact::Interactor,
    job::Jobs,
    keymap::{KeymapId, Keymaps},
    terminal::Terminal,
    ui::{
        self,
        components::{
            authentication::AuthenticateState, filter::FeedFilter, root::Root, tabs::Tab,
            Components,
        },
        theme::Theme,
    },
};

mod direction;
pub use direction::{Direction, IndexOutOfRange};

mod in_flight;
pub use in_flight::{InFlight, RequestId, RequestSequence};

mod input_parser;
pub use input_parser::parse_requirement;
use input_parser::InputParser;

mod authenticator;
pub use authenticator::{Authenticator, DeviceFlows, JwtService};

enum Screen {
    Login,
    Browse,
}

#[derive(PartialEq, Eq)]
pub enum EventLoopControlFlow {
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListAction {
    Append,
    Replace,
}

pub struct Config {
    pub idle_timer_interval: Duration,
    pub throbber_timer_interval: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle_timer_interval: Duration::from_secs(250),
            throbber_timer_interval: Duration::from_millis(250),
        }
    }
}

pub struct Application {
    terminal: Terminal,
    client: Client,
    jobs: Jobs,
    components: Components,
    interactor: Interactor,
    authenticator: Authenticator,
    in_flight: InFlight,
    theme: Theme,
    idle_timer: Pin<Box<Sleep>>,
    config: Config,
    keymaps: Keymaps,
    categories: Categories,

    screen: Screen,
    should_render: bool,
    should_quit: bool,
}

impl Application {
    pub fn new(terminal: Terminal, client: Client, categories: Categories) -> Self {
        Application::with(terminal, client, categories, Config::default())
    }

    pub fn with(
        terminal: Terminal,
        client: Client,
        categories: Categories,
        config: Config,
    ) -> Self {
        let mut keymaps = Keymaps::default();
        keymaps.enable(KeymapId::Global);
        keymaps.enable(KeymapId::Login);

        Self {
            terminal,
            client,
            jobs: Jobs::new(),
            components: Components::new(),
            interactor: Interactor::new(),
            authenticator: Authenticator::new(),
            in_flight: InFlight::new().with_throbber_timer_interval(config.throbber_timer_interval),
            theme: Theme::with_palette(&tailwind::BLUE),
            idle_timer: Box::pin(tokio::time::sleep(config.idle_timer_interval)),
            screen: Screen::Login,
            config,
            keymaps,
            categories,
            should_quit: false,
            should_render: false,
        }
    }

    #[must_use]
    pub fn with_theme(self, theme: Theme) -> Self {
        Self { theme, ..self }
    }

    #[must_use]
    pub fn with_authenticator(self, authenticator: Authenticator) -> Self {
        Self {
            authenticator,
            ..self
        }
    }

    pub fn jwt_service(&self) -> &JwtService {
        &self.authenticator.jwt_service
    }

    pub fn set_credential(&mut self, cred: Credential) {
        self.client.set_credential(cred);
        self.components.auth.authenticated();
        self.keymaps.disable(KeymapId::Login);
        self.keymaps.enable(KeymapId::Tabs);
        self.keymaps.enable(KeymapId::Entries);
        self.keymaps.enable(KeymapId::Filter);
        self.initial_fetch();
        self.screen = Screen::Browse;
        self.should_render = true;
        self.reset_idle_timer();
    }

    fn initial_fetch(&mut self) {
        tracing::info!("Initial fetch");
        let fut = async {
            Ok(Command::FetchEntries {
                after: None,
                first: config::client::INITIAL_ENTRIES_TO_FETCH,
            })
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    pub async fn run<S>(mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.terminal.init()?;

        self.event_loop(input).await;

        self.terminal.restore()?;

        Ok(())
    }

    async fn event_loop<S>(&mut self, input: &mut S)
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.render();

        loop {
            if self.event_loop_until_idle(input).await == EventLoopControlFlow::Quit {
                break;
            }
        }
    }

    pub async fn event_loop_until_idle<S>(&mut self, input: &mut S) -> EventLoopControlFlow
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        loop {
            let command = tokio::select! {
                biased;

                Some(event) = input.next() => {
                    self.handle_terminal_event(event)
                }
                Some(command) = self.jobs.futures.next() => {
                    Some(command.unwrap())
                }
                ()  = self.in_flight.throbber_timer() => {
                    Some(Command::RenderThrobber)
                }
                () = &mut self.idle_timer => {
                    Some(Command::Idle)
                }
            };

            if let Some(command) = command {
                self.apply(command);
            }

            if self.should_render {
                self.render();
                self.should_render = false;
                self.components.prompt.clear_error_message();
            }

            if self.should_quit {
                self.should_quit = false; // for testing
                break EventLoopControlFlow::Quit;
            }
        }
    }

    #[tracing::instrument(skip_all,fields(%command))]
    fn apply(&mut self, command: Command) {
        let mut next = Some(command);

        // should detect infinite loop ?
        while let Some(command) = next.take() {
            match command {
                Command::Quit => self.should_quit = true,
                Command::ResizeTerminal { .. } => {
                    self.should_render = true;
                }
                Command::RenderThrobber => {
                    self.in_flight.reset_throbber_timer();
                    self.in_flight.inc_throbber_step();
                    self.should_render = true;
                }
                Command::Idle => {
                    self.handle_idle();
                }
                Command::Authenticate => {
                    if self.components.auth.state() != &AuthenticateState::NotAuthenticated {
                        continue;
                    }
                    let provider = self.components.auth.selected_provider();
                    match provider {
                        AuthenticationProvider::Github => {
                            self.authenticate(
                                provider,
                                self.authenticator.device_flows.github.clone(),
                            );
                        }
                        AuthenticationProvider::Google => {
                            self.authenticate(
                                provider,
                                self.authenticator.device_flows.google.clone(),
                            );
                        }
                    }
                }
                Command::MoveAuthenticationProvider(direction) => {
                    self.components.auth.move_selection(&direction);
                    self.should_render = true;
                }
                Command::DeviceAuthorizationFlow {
                    provider,
                    device_authorization,
                } => match provider {
                    AuthenticationProvider::Github => {
                        self.device_authorize_flow(
                            provider,
                            self.authenticator.device_flows.github.clone(),
                            device_authorization,
                        );
                    }
                    AuthenticationProvider::Google => {
                        self.device_authorize_flow(
                            provider,
                            self.authenticator.device_flows.google.clone(),
                            device_authorization,
                        );
                    }
                },
                Command::CompleteDevieAuthorizationFlow {
                    provider,
                    device_access_token,
                } => {
                    self.complete_device_authroize_flow(provider, device_access_token);
                }
                Command::MoveTabSelection(direction) => {
                    self.keymaps.toggle(KeymapId::Entries);
                    self.keymaps.toggle(KeymapId::Subscription);

                    match self.components.tabs.move_selection(&direction) {
                        Tab::Feeds if !self.components.subscription.has_subscription() => {
                            next = Some(Command::FetchSubscription {
                                after: None,
                                first: config::client::INITIAL_FEEDS_TO_FETCH,
                            });
                        }
                        _ => {}
                    }
                    self.should_render = true;
                }
                Command::MoveSubscribedFeed(direction) => {
                    self.components.subscription.move_selection(&direction);
                    self.should_render = true;
                }
                Command::MoveSubscribedFeedFirst => {
                    self.components.subscription.move_first();
                    self.should_render = true;
                }
                Command::MoveSubscribedFeedLast => {
                    self.components.subscription.move_last();
                    self.should_render = true;
                }
                Command::PromptFeedSubscription => {
                    self.prompt_feed_subscription();
                    self.should_render = true;
                }
                Command::PromptFeedEdition => {
                    self.prompt_feed_edition();
                    self.should_render = true;
                }
                Command::PromptFeedUnsubscription => {
                    self.prompt_feed_unsubscription();
                    self.should_render = true;
                }
                Command::SubscribeFeed { input } => {
                    self.subscribe_feed(input);
                    self.should_render = true;
                }
                Command::UnsubscribeFeed { url } => {
                    self.unsubscribe_feed(url);
                    self.should_render = true;
                }
                Command::FetchSubscription { after, first } => {
                    self.fetch_subscription(ListAction::Append, after, first);
                }
                Command::UpdateSubscriptionState {
                    action,
                    subscription,
                    request_seq,
                } => {
                    self.in_flight.remove(request_seq);
                    self.components
                        .subscription
                        .update_subscription(action, subscription);
                    self.should_render = true;
                }
                Command::ReloadSubscription => {
                    self.fetch_subscription(
                        ListAction::Replace,
                        None,
                        config::client::INITIAL_FEEDS_TO_FETCH,
                    );
                    self.should_render = true;
                }
                Command::CompleteSubscribeFeed { feed, request_seq } => {
                    self.in_flight.remove(request_seq);
                    self.components.subscription.upsert_subscribed_feed(feed);
                    self.fetch_entries(
                        ListAction::Replace,
                        None,
                        config::client::INITIAL_ENTRIES_TO_FETCH,
                    );
                    self.should_render = true;
                }
                Command::CompleteUnsubscribeFeed { url, request_seq } => {
                    self.in_flight.remove(request_seq);
                    self.components.subscription.remove_unsubscribed_feed(&url);
                    self.components.entries.remove_unsubscribed_entries(&url);
                    self.components.filter.update_categories(
                        &self.categories,
                        ListAction::Replace,
                        self.components.entries.entries(),
                    );
                    self.should_render = true;
                }
                Command::OpenFeed => {
                    self.open_feed();
                }
                Command::FetchEntries { after, first } => {
                    self.fetch_entries(ListAction::Append, after, first);
                }
                Command::UpdateEntriesState {
                    action,
                    payload,
                    request_seq,
                } => {
                    self.in_flight.remove(request_seq);
                    self.components.filter.update_categories(
                        &self.categories,
                        action,
                        payload.entries.as_slice(),
                    );
                    self.components.entries.update_entries(action, payload);
                    self.should_render = true;
                }
                Command::ReloadEntries => {
                    self.fetch_entries(
                        ListAction::Replace,
                        None,
                        config::client::INITIAL_ENTRIES_TO_FETCH,
                    );
                    self.should_render = true;
                }
                Command::MoveEntry(direction) => {
                    self.components.entries.move_selection(&direction);
                    self.should_render = true;
                }
                Command::MoveEntryFirst => {
                    self.components.entries.move_first();
                    self.should_render = true;
                }
                Command::MoveEntryLast => {
                    self.components.entries.move_last();
                    self.should_render = true;
                }
                Command::OpenEntry => {
                    self.open_entry();
                }
                Command::MoveFilterRequirement(direction) => {
                    let filter = self.components.filter.move_requirement(direction);
                    self.apply_feed_filter(filter);
                    self.should_render = true;
                }
                Command::ActivateCategoryFilterling => {
                    let keymap = self.components.filter.activate_category_filtering();
                    self.keymaps.update(KeymapId::CategoryFiltering, keymap);
                    self.should_render = true;
                }
                Command::DeactivateCategoryFiltering => {
                    self.components.filter.deactivate_category_filtering();
                    self.keymaps.disable(KeymapId::CategoryFiltering);
                    self.should_render = true;
                }
                Command::ToggleFilterCategory { category } => {
                    let filter = self.components.filter.toggle_category_state(&category);
                    self.apply_feed_filter(filter);
                    self.should_render = true;
                }
                Command::ActivateAllFilterCategories => {
                    let filter = self.components.filter.activate_all_categories_state();
                    self.apply_feed_filter(filter);
                    self.should_render = true;
                }
                Command::DeactivateAllFilterCategories => {
                    let filter = self.components.filter.deactivate_all_categories_state();
                    self.apply_feed_filter(filter);
                    self.should_render = true;
                }
                Command::HandleError {
                    message,
                    request_seq,
                } => {
                    tracing::error!("{message}");

                    if let Some(request_seq) = request_seq {
                        self.in_flight.remove(request_seq);
                    }
                    self.components.prompt.set_error_message(message);
                    self.should_render = true;
                }
            }
        }
    }

    fn render(&mut self) {
        let cx = ui::Context {
            theme: &self.theme,
            in_flight: &self.in_flight,
            categories: &self.categories,
        };
        let root = Root::new(&self.components, cx);

        self.terminal
            .render(|frame| Widget::render(root, frame.size(), frame.buffer_mut()))
            .expect("Failed to render");
    }

    fn handle_terminal_event(&mut self, event: std::io::Result<CrosstermEvent>) -> Option<Command> {
        match event.unwrap() {
            CrosstermEvent::Resize(columns, rows) => {
                Some(Command::ResizeTerminal { columns, rows })
            }
            CrosstermEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            }) => None,
            CrosstermEvent::Key(key) => {
                tracing::debug!("Handle key event: {key:?}");

                self.reset_idle_timer();
                self.keymaps.search(key)
            }
            _ => None,
        }
    }
}

impl Application {
    fn fetch_subscription(&mut self, action: ListAction, after: Option<String>, first: i64) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchSubscription);
        let fut = async move {
            match client.fetch_subscription(after, Some(first)).await {
                Ok(subscription) => Ok(Command::UpdateSubscriptionState {
                    action,
                    subscription,
                    request_seq,
                }),
                Err(err) => Ok(Command::HandleError {
                    message: format!("{err}"),
                    request_seq: Some(request_seq),
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }
}

impl Application {
    fn prompt_feed_subscription(&mut self) {
        let input = self
            .interactor
            .open_editor(InputParser::SUSBSCRIBE_FEED_PROMPT);
        tracing::debug!("Got user modified feed subscription: {input}");
        // the terminal state becomes strange after editing in the editor
        self.terminal.force_redraw();

        let fut = match InputParser::new(input.as_str()).parse_feed_subscription(&self.categories) {
            Ok(input) => {
                // Check for the duplicate subscription
                if self
                    .components
                    .subscription
                    .is_already_subscribed(&input.url)
                {
                    let message = format!("{} already subscribed", input.url);
                    future::ready(Ok(Command::HandleError {
                        message,
                        request_seq: None,
                    }))
                    .boxed()
                } else {
                    future::ready(Ok(Command::SubscribeFeed { input })).boxed()
                }
            }

            Err(err) => async move {
                Ok(Command::HandleError {
                    message: err.to_string(),
                    request_seq: None,
                })
            }
            .boxed(),
        };

        self.jobs.futures.push(fut);
    }

    fn prompt_feed_edition(&mut self) {
        let Some(feed) = self.components.subscription.selected_feed() else {
            return;
        };

        let input = self
            .interactor
            .open_editor(InputParser::edit_feed_prompt(feed));
        // the terminal state becomes strange after editing in the editor
        self.terminal.force_redraw();

        let fut = match InputParser::new(input.as_str()).parse_feed_subscription(&self.categories) {
            // Strictly, if the URL of the feed changed before and after an update
            // it is not considered an edit, so it could be considered an error
            // but currently we are allowing it
            Ok(input) => async move { Ok(Command::SubscribeFeed { input }) }.boxed(),
            Err(err) => async move {
                Ok(Command::HandleError {
                    message: err.to_string(),
                    request_seq: None,
                })
            }
            .boxed(),
        };

        self.jobs.futures.push(fut);
    }

    fn prompt_feed_unsubscription(&mut self) {
        // TODO: prompt deletion confirm
        let Some(url) = self
            .components
            .subscription
            .selected_feed()
            .map(|feed| feed.url.clone())
        else {
            return;
        };
        let fut = async move { Ok(Command::UnsubscribeFeed { url }) }.boxed();
        self.jobs.futures.push(fut);
    }

    fn subscribe_feed(&mut self, input: SubscribeFeedInput) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::SubscribeFeed);
        let fut = async move {
            match client.subscribe_feed(input).await {
                Ok(feed) => Ok(Command::CompleteSubscribeFeed { feed, request_seq }),
                Err(err) => Ok(Command::HandleError {
                    message: format!("{err}"),
                    request_seq: Some(request_seq),
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    fn unsubscribe_feed(&mut self, url: String) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::UnsubscribeFeed);
        let fut = async move {
            match client.unsubscribe_feed(url.clone()).await {
                Ok(()) => Ok(Command::CompleteUnsubscribeFeed { url, request_seq }),
                Err(err) => Ok(Command::HandleError {
                    message: format!("{err}"),
                    request_seq: Some(request_seq),
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }
}

impl Application {
    fn open_feed(&mut self) {
        let Some(feed_website_url) = self
            .components
            .subscription
            .selected_feed()
            .and_then(|feed| feed.website_url.clone())
        else {
            return;
        };
        self.interactor.open_browser(feed_website_url);
    }

    fn open_entry(&mut self) {
        let Some(entry_website_url) = self.components.entries.selected_entry_website_url() else {
            return;
        };
        self.interactor.open_browser(entry_website_url);
    }
}

impl Application {
    #[tracing::instrument(skip(self))]
    fn fetch_entries(&mut self, action: ListAction, after: Option<String>, first: i64) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchEntries);
        let fut = async move {
            match client.fetch_entries(after, first).await {
                Ok(payload) => Ok(Command::UpdateEntriesState {
                    action,
                    payload,
                    request_seq,
                }),
                Err(err) => Ok(Command::HandleError {
                    message: format!("{err}"),
                    request_seq: Some(request_seq),
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }
}

impl Application {
    #[tracing::instrument(skip(self, device_flow))]
    fn authenticate<P>(&mut self, provider: AuthenticationProvider, device_flow: DeviceFlow<P>)
    where
        P: device_flow::Provider + Sync + Send + 'static,
    {
        tracing::info!("Start authenticate");

        let fut = async move {
            match device_flow.device_authorize_request().await {
                Ok(device_authorization) => Ok(Command::DeviceAuthorizationFlow {
                    provider,
                    device_authorization,
                }),
                Err(err) => Ok(Command::HandleError {
                    message: format!("{err}"),
                    request_seq: None,
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    fn device_authorize_flow<P>(
        &mut self,
        provider: AuthenticationProvider,
        device_flow: DeviceFlow<P>,
        device_authorization: DeviceAuthorizationResponse,
    ) where
        P: device_flow::Provider + Sync + Send + 'static,
    {
        self.components
            .auth
            .set_device_authorization_response(device_authorization.clone());
        self.should_render = true;
        // try to open input screen in the browser
        self.interactor
            .open_browser(device_authorization.verification_uri().to_string());

        let fut = async move {
            match device_flow
                .poll_device_access_token(
                    device_authorization.device_code,
                    device_authorization.interval,
                )
                .await
            {
                Ok(device_access_token) => Ok(Command::CompleteDevieAuthorizationFlow {
                    provider,
                    device_access_token,
                }),
                Err(err) => Ok(Command::HandleError {
                    message: format!("{err}"),
                    request_seq: None,
                }),
            }
        }
        .boxed();

        self.jobs.futures.push(fut);
    }

    fn complete_device_authroize_flow(
        &mut self,
        provider: AuthenticationProvider,
        device_access_token: DeviceAccessTokenResponse,
    ) {
        let auth = match provider {
            AuthenticationProvider::Github => Credential::Github {
                access_token: device_access_token.access_token,
            },
            AuthenticationProvider::Google => Credential::Google {
                id_token: device_access_token.id_token.expect("id token not found"),
                refresh_token: device_access_token
                    .refresh_token
                    .expect("refresh token not found"),
            },
        };

        // should test with tmp file?
        #[cfg(not(feature = "integration"))]
        {
            if let Err(err) = crate::auth::persist_credential(&auth) {
                tracing::warn!("Failed to save credential cache: {err}");
            }
        }

        self.set_credential(auth);
    }
}

impl Application {
    fn apply_feed_filter(&mut self, filter: FeedFilter) {
        self.components.entries.update_filter(filter.clone());
        self.components.subscription.update_filter(filter);
    }
}

impl Application {
    fn handle_idle(&mut self) {
        self.clear_idle_timer();

        #[cfg(feature = "integration")]
        {
            tracing::debug!("Quit for idle");
            self.should_render = true;
            self.should_quit = true;
        }
    }

    pub fn clear_idle_timer(&mut self) {
        // https://github.com/tokio-rs/tokio/blob/e53b92a9939565edb33575fff296804279e5e419/tokio/src/time/instant.rs#L62
        self.idle_timer
            .as_mut()
            .reset(Instant::now() + Duration::from_secs(86400 * 365 * 30));
    }

    pub fn reset_idle_timer(&mut self) {
        self.idle_timer
            .as_mut()
            .reset(Instant::now() + self.config.idle_timer_interval);
    }
}

#[cfg(feature = "integration")]
impl Application {
    pub fn assert_buffer(&self, expected: &ratatui::buffer::Buffer) {
        self.terminal.assert_buffer(expected);
    }
}
