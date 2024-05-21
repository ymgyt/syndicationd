use std::{
    future,
    ops::{Add, ControlFlow, Sub},
    pin::Pin,
    time::Duration,
};

use chrono::{DateTime, Utc};
use crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind};
use futures_util::{FutureExt, Stream, StreamExt};
use ratatui::{style::palette::tailwind, widgets::Widget};
use synd_auth::device_flow::{
    self, DeviceAccessTokenResponse, DeviceAuthorizationResponse, DeviceFlow,
};
use synd_feed::types::FeedUrl;
use tokio::time::{Instant, Sleep};

use crate::{
    application::event::KeyEventResult,
    auth::{self, AuthenticationProvider, Credential},
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
            authentication::AuthenticateState, filter::FeedFilter, root::Root,
            subscription::UnsubscribeSelection, tabs::Tab, Components,
        },
        theme::Theme,
    },
};

mod direction;
pub(crate) use direction::{Direction, IndexOutOfRange};

mod in_flight;
pub(crate) use in_flight::{InFlight, RequestId, RequestSequence};

mod input_parser;
use input_parser::InputParser;

mod authenticator;
pub use authenticator::{Authenticator, DeviceFlows, JwtService};

mod clock;
pub(crate) use clock::{Clock, SystemClock};

mod flags;
use flags::Should;

pub(crate) mod event;

enum Screen {
    Login,
    Browse,
}

#[derive(PartialEq, Eq)]
pub enum EventLoopControlFlow {
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Populate {
    Append,
    Replace,
}

pub struct Config {
    pub idle_timer_interval: Duration,
    pub throbber_timer_interval: Duration,
    pub entries_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle_timer_interval: Duration::from_secs(250),
            throbber_timer_interval: Duration::from_millis(250),
            entries_limit: config::feed::DEFAULT_ENTRIES_LIMIT,
        }
    }
}

pub struct Application {
    clock: Box<dyn Clock>,
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
    key_handlers: event::KeyHandlers,
    categories: Categories,

    screen: Screen,
    flags: Should,
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

        let mut key_handlers = event::KeyHandlers::new();
        key_handlers.push(event::KeyHandler::Keymaps(keymaps));

        Self {
            clock: Box::new(SystemClock),
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
            key_handlers,
            categories,
            flags: Should::empty(),
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

    fn now(&self) -> DateTime<Utc> {
        self.clock.now()
    }

    fn jwt_service(&self) -> &JwtService {
        &self.authenticator.jwt_service
    }

    fn keymaps(&mut self) -> &mut Keymaps {
        self.key_handlers.keymaps_mut().unwrap()
    }

    pub async fn restore_credential(&self) -> Option<Credential> {
        auth::credential_from_cache(self.jwt_service(), self.now()).await
    }

    pub fn handle_initial_credential(&mut self, cred: Credential) {
        self.set_credential(cred);
        self.initial_fetch();
        self.components.auth.authenticated();
        self.keymaps().disable(KeymapId::Login);
        self.keymaps().enable(KeymapId::Tabs);
        self.keymaps().enable(KeymapId::Entries);
        self.keymaps().enable(KeymapId::Filter);
        self.screen = Screen::Browse;
        self.reset_idle_timer();
        self.should_render();
    }

    fn set_credential(&mut self, cred: Credential) {
        self.schedule_credential_refreshing(&cred);
        self.client.set_credential(cred);
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
            if self.event_loop_until_idle(input).await.is_break() {
                break;
            }
        }
    }

    pub async fn event_loop_until_idle<S>(&mut self, input: &mut S) -> ControlFlow<()>
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

            if self.flags.contains(Should::Render) {
                self.render();
                self.flags.remove(Should::Render);
                self.components.prompt.clear_error_message();
            }

            if self.flags.contains(Should::Quit) {
                self.flags.remove(Should::Quit); // for testing
                break ControlFlow::Break(());
            }
        }
    }

    #[tracing::instrument(skip_all,fields(%command))]
    fn apply(&mut self, command: Command) {
        let mut next = Some(command);

        // should detect infinite loop ?
        while let Some(command) = next.take() {
            match command {
                Command::Quit => self.flags.insert(Should::Quit),
                Command::ResizeTerminal { .. } => {
                    self.should_render();
                }
                Command::RenderThrobber => {
                    self.in_flight.reset_throbber_timer();
                    self.in_flight.inc_throbber_step();
                    self.should_render();
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
                    self.components.auth.move_selection(direction);
                    self.should_render();
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
                Command::RefreshCredential { credential } => {
                    self.set_credential(credential);
                }
                Command::MoveTabSelection(direction) => {
                    self.keymaps().toggle(KeymapId::Entries);
                    self.keymaps().toggle(KeymapId::Subscription);

                    match self.components.tabs.move_selection(direction) {
                        Tab::Feeds if !self.components.subscription.has_subscription() => {
                            next = Some(Command::FetchSubscription {
                                after: None,
                                first: config::client::INITIAL_FEEDS_TO_FETCH,
                            });
                        }
                        _ => {}
                    }
                    self.should_render();
                }
                Command::MoveSubscribedFeed(direction) => {
                    self.components.subscription.move_selection(direction);
                    self.should_render();
                }
                Command::MoveSubscribedFeedFirst => {
                    self.components.subscription.move_first();
                    self.should_render();
                }
                Command::MoveSubscribedFeedLast => {
                    self.components.subscription.move_last();
                    self.should_render();
                }
                Command::PromptFeedSubscription => {
                    self.prompt_feed_subscription();
                    self.should_render();
                }
                Command::PromptFeedEdition => {
                    self.prompt_feed_edition();
                    self.should_render();
                }
                Command::PromptFeedUnsubscription => {
                    if self.components.subscription.selected_feed().is_some() {
                        self.components.subscription.show_unsubscribe_popup(true);
                        self.keymaps().enable(KeymapId::UnsubscribePopupSelection);
                        self.should_render();
                    }
                }
                Command::MoveFeedUnsubscriptionPopupSelection(direction) => {
                    self.components
                        .subscription
                        .move_unsubscribe_popup_selection(direction);
                    self.should_render();
                }
                Command::SelectFeedUnsubscriptionPopup => {
                    if let (UnsubscribeSelection::Yes, Some(feed)) =
                        self.components.subscription.unsubscribe_popup_selection()
                    {
                        self.unsubscribe_feed(feed.url.clone());
                    }
                    next = Some(Command::CancelFeedUnsubscriptionPopup);
                    self.should_render();
                }
                Command::CancelFeedUnsubscriptionPopup => {
                    self.components.subscription.show_unsubscribe_popup(false);
                    self.keymaps().disable(KeymapId::UnsubscribePopupSelection);
                    self.should_render();
                }
                Command::SubscribeFeed { input } => {
                    self.subscribe_feed(input);
                    self.should_render();
                }
                Command::FetchSubscription { after, first } => {
                    self.fetch_subscription(Populate::Append, after, first);
                }
                Command::PopulateFetchedSubscription {
                    populate,
                    subscription,
                    request_seq,
                } => {
                    self.in_flight.remove(request_seq);
                    // paginate
                    next = subscription.feeds.page_info.has_next_page.then(|| {
                        Command::FetchSubscription {
                            after: subscription.feeds.page_info.end_cursor.clone(),
                            first: subscription.feeds.nodes.len().try_into().unwrap_or(0),
                        }
                    });
                    // how we show fetched errors in ui?
                    if !subscription.feeds.errors.is_empty() {
                        tracing::warn!("Failed fetched feeds: {:?}", subscription.feeds.errors);
                    }
                    self.components
                        .subscription
                        .update_subscription(populate, subscription);
                    self.should_render();
                }
                Command::ReloadSubscription => {
                    self.fetch_subscription(
                        Populate::Replace,
                        None,
                        config::client::INITIAL_FEEDS_TO_FETCH,
                    );
                    self.should_render();
                }
                Command::CompleteSubscribeFeed { feed, request_seq } => {
                    self.in_flight.remove(request_seq);
                    self.components.subscription.upsert_subscribed_feed(feed);
                    self.fetch_entries(
                        Populate::Replace,
                        None,
                        config::client::INITIAL_ENTRIES_TO_FETCH,
                    );
                    self.should_render();
                }
                Command::CompleteUnsubscribeFeed { url, request_seq } => {
                    self.in_flight.remove(request_seq);
                    self.components.subscription.remove_unsubscribed_feed(&url);
                    self.components.entries.remove_unsubscribed_entries(&url);
                    self.components.filter.update_categories(
                        &self.categories,
                        Populate::Replace,
                        self.components.entries.entries(),
                    );
                    self.should_render();
                }
                Command::OpenFeed => {
                    self.open_feed();
                }
                Command::FetchEntries { after, first } => {
                    self.fetch_entries(Populate::Append, after, first);
                }
                Command::PopulateFetchedEntries {
                    populate,
                    payload,
                    request_seq,
                } => {
                    self.in_flight.remove(request_seq);
                    self.components.filter.update_categories(
                        &self.categories,
                        populate,
                        payload.entries.as_slice(),
                    );
                    // paginate
                    next = payload
                        .page_info
                        .has_next_page
                        .then(|| Command::FetchEntries {
                            after: payload.page_info.end_cursor.clone(),
                            first: self
                                .config
                                .entries_limit
                                .saturating_sub(
                                    self.components.entries.count() + payload.entries.len(),
                                )
                                .min(payload.entries.len())
                                .try_into()
                                .unwrap_or(0),
                        });
                    self.components.entries.update_entries(populate, payload);
                    self.should_render();
                }
                Command::ReloadEntries => {
                    self.fetch_entries(
                        Populate::Replace,
                        None,
                        config::client::INITIAL_ENTRIES_TO_FETCH,
                    );
                    self.should_render();
                }
                Command::MoveEntry(direction) => {
                    self.components.entries.move_selection(direction);
                    self.should_render();
                }
                Command::MoveEntryFirst => {
                    self.components.entries.move_first();
                    self.should_render();
                }
                Command::MoveEntryLast => {
                    self.components.entries.move_last();
                    self.should_render();
                }
                Command::OpenEntry => {
                    self.open_entry();
                }
                Command::MoveFilterRequirement(direction) => {
                    let filter = self.components.filter.move_requirement(direction);
                    self.apply_feed_filter(filter);
                    self.should_render();
                }
                Command::ActivateCategoryFilterling => {
                    let keymap = self.components.filter.activate_category_filtering();
                    self.keymaps().update(KeymapId::CategoryFiltering, keymap);
                    self.should_render();
                }
                Command::ActivateSearchFiltering => {
                    let prompt = self.components.filter.activate_search_filtering();
                    self.key_handlers.push(event::KeyHandler::Prompt(prompt));
                    self.should_render();
                }
                Command::PromptChanged => {
                    if self.components.filter.is_search_active() {
                        let filter = self.components.filter.feed_filter();
                        self.apply_feed_filter(filter);
                        self.should_render();
                    }
                }
                Command::DeactivateFiltering => {
                    self.components.filter.deactivate_filtering();
                    self.keymaps().disable(KeymapId::CategoryFiltering);
                    self.key_handlers.remove_prompt();
                    self.should_render();
                }
                Command::ToggleFilterCategory { category } => {
                    let filter = self.components.filter.toggle_category_state(&category);
                    self.apply_feed_filter(filter);
                    self.should_render();
                }
                Command::ActivateAllFilterCategories => {
                    let filter = self.components.filter.activate_all_categories_state();
                    self.apply_feed_filter(filter);
                    self.should_render();
                }
                Command::DeactivateAllFilterCategories => {
                    let filter = self.components.filter.deactivate_all_categories_state();
                    self.apply_feed_filter(filter);
                    self.should_render();
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
                    self.should_render();
                }
            }
        }
    }

    #[inline]
    fn should_render(&mut self) {
        self.flags.insert(Should::Render);
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
            CrosstermEvent::Resize(columns, rows) => Some(Command::ResizeTerminal {
                _columns: columns,
                _rows: rows,
            }),
            CrosstermEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            }) => None,
            CrosstermEvent::Key(key) => {
                tracing::debug!("Handle key event: {key:?}");

                self.reset_idle_timer();

                match self.key_handlers.handle(key) {
                    KeyEventResult::Consumed(cmd) => {
                        self.should_render();
                        cmd
                    }
                    KeyEventResult::Ignored => None,
                }
            }
            _ => None,
        }
    }
}

impl Application {
    fn fetch_subscription(&mut self, populate: Populate, after: Option<String>, first: i64) {
        if first <= 0 {
            return;
        }
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchSubscription);
        let fut = async move {
            match client.fetch_subscription(after, Some(first)).await {
                Ok(subscription) => Ok(Command::PopulateFetchedSubscription {
                    populate,
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

    fn unsubscribe_feed(&mut self, url: FeedUrl) {
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
    fn fetch_entries(&mut self, populate: Populate, after: Option<String>, first: i64) {
        if first <= 0 {
            return;
        }
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchEntries);
        let fut = async move {
            match client.fetch_entries(after, first).await {
                Ok(payload) => Ok(Command::PopulateFetchedEntries {
                    populate,
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
        self.should_render();
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
            AuthenticationProvider::Google => {
                let id_token = device_access_token.id_token.expect("id token not found");
                let expired_at = self
                    .jwt_service()
                    .google
                    .decode_id_token_insecure(&id_token, false)
                    .ok()
                    .map_or(
                        self.now().add(config::credential::FALLBACK_EXPIRE),
                        |claims| claims.expired_at(),
                    );
                Credential::Google {
                    id_token,
                    refresh_token: device_access_token
                        .refresh_token
                        .expect("refresh token not found"),
                    expired_at,
                }
            }
        };

        // should test with tmp file?
        #[cfg(not(feature = "integration"))]
        {
            if let Err(err) = crate::auth::persist_credential(&auth) {
                tracing::warn!("Failed to save credential cache: {err}");
            }
        }

        self.handle_initial_credential(auth);
    }

    fn schedule_credential_refreshing(&mut self, cred: &Credential) {
        match cred {
            Credential::Github { .. } => {}
            Credential::Google {
                refresh_token,
                expired_at,
                ..
            } => {
                let until_expire = expired_at
                    .sub(config::credential::EXPIRE_MARGIN)
                    .sub(self.now())
                    .to_std()
                    .unwrap_or(config::credential::FALLBACK_EXPIRE);
                let jwt_service = self.jwt_service().clone();
                let refresh_token = refresh_token.clone();
                let fut = async move {
                    tokio::time::sleep(until_expire).await;

                    tracing::debug!("Refresh google credential");
                    match jwt_service.refresh_google_id_token(&refresh_token).await {
                        Ok(credential) => Ok(Command::RefreshCredential { credential }),
                        Err(err) => Ok(Command::HandleError {
                            message: err.to_string(),
                            request_seq: None,
                        }),
                    }
                }
                .boxed();
                self.jobs.futures.push(fut);
            }
        }
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
            self.should_render();
            self.flags.insert(Should::Quit);
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
