use std::{
    collections::VecDeque,
    future,
    ops::{ControlFlow, Sub},
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use chrono::{DateTime, Utc};
use crossterm::event::{Event as CrosstermEvent, KeyEvent, KeyEventKind};
use either::Either;
use futures_util::{FutureExt, Stream, StreamExt};
use itertools::Itertools;
use ratatui::widgets::Widget;
use synd_auth::device_flow::DeviceAuthorizationResponse;
use synd_feed::types::FeedUrl;
use tokio::time::{Instant, Sleep};
use update_informer::Version;

use crate::{
    application::event::KeyEventResult,
    auth::{self, AuthenticationProvider, Credential, CredentialError, Verified},
    client::{
        github::{FetchNotificationInclude, FetchNotificationsParams, GithubClient},
        mutation::subscribe_feed::SubscribeFeedInput,
        Client, SyndApiError,
    },
    command::{ApiResponse, Command},
    config::{self, Categories},
    interact::Interactor,
    job::Jobs,
    keymap::{KeymapId, Keymaps},
    terminal::Terminal,
    types::github::{IssueOrPullRequest, Notification},
    ui::{
        self,
        components::{
            authentication::AuthenticateState, filter::Filterer, root::Root,
            subscription::UnsubscribeSelection, tabs::Tab, Components,
        },
        theme::{Palette, Theme},
    },
};

mod direction;
pub(crate) use direction::{Direction, IndexOutOfRange};

mod in_flight;
pub(crate) use in_flight::{InFlight, RequestId, RequestSequence};

mod input_parser;
use input_parser::InputParser;

pub use auth::authenticator::{Authenticator, DeviceFlows, JwtService};

mod clock;
pub(crate) use clock::{Clock, SystemClock};

mod cache;
pub use cache::Cache;

mod builder;
pub use builder::ApplicationBuilder;

mod app_config;
pub use app_config::{Config, Features};

pub(crate) mod event;

mod state;
pub(crate) use state::TerminalFocus;
use state::{Should, State};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Populate {
    Append,
    Replace,
}

pub struct Application {
    clock: Box<dyn Clock>,
    terminal: Terminal,
    client: Client,
    github_client: Option<GithubClient>,
    jobs: Jobs,
    components: Components,
    interactor: Interactor,
    authenticator: Authenticator,
    in_flight: InFlight,
    cache: Cache,
    theme: Theme,
    idle_timer: Pin<Box<Sleep>>,
    config: Config,
    key_handlers: event::KeyHandlers,
    categories: Categories,
    latest_release: Option<Version>,

    state: State,
}

impl Application {
    /// Construct `ApplicationBuilder`
    pub fn builder() -> ApplicationBuilder {
        ApplicationBuilder::default()
    }

    /// Construct `Application` from builder.
    /// Configure keymaps for terminal use
    fn new(
        builder: ApplicationBuilder<Terminal, Client, Categories, Cache, Config, Theme>,
    ) -> Self {
        let ApplicationBuilder {
            terminal,
            client,
            github_client,
            categories,
            cache,
            config,
            theme,
            authenticator,
            interactor,
            dry_run,
        } = builder;

        let key_handlers = {
            let mut keymaps = Keymaps::default();
            keymaps.enable(KeymapId::Global);
            keymaps.enable(KeymapId::Login);

            let mut key_handlers = event::KeyHandlers::new();
            key_handlers.push(event::KeyHandler::Keymaps(keymaps));
            key_handlers
        };
        let mut state = State::new();
        if dry_run {
            state.flags = Should::Quit;
        }

        Self {
            clock: Box::new(SystemClock),
            terminal,
            client,
            github_client,
            jobs: Jobs::new(),
            components: Components::new(&config.features),
            interactor: interactor.unwrap_or_else(Interactor::new),
            authenticator: authenticator.unwrap_or_else(Authenticator::new),
            in_flight: InFlight::new().with_throbber_timer_interval(config.throbber_timer_interval),
            cache,
            theme,
            idle_timer: Box::pin(tokio::time::sleep(config.idle_timer_interval)),
            config,
            key_handlers,
            categories,
            latest_release: None,
            state,
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

    pub async fn run<S>(mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.init().await?;

        self.event_loop(input).await;

        self.cleanup().ok();

        Ok(())
    }

    /// Initialize application.
    /// Setup terminal and handle cache.
    async fn init(&mut self) -> anyhow::Result<()> {
        match self.terminal.init() {
            Ok(()) => Ok(()),
            Err(err) => {
                if self.state.flags.contains(Should::Quit) {
                    tracing::warn!("Failed to init terminal: {err}");
                    Ok(())
                } else {
                    Err(err)
                }
            }
        }?;

        match self.restore_credential().await {
            Ok(cred) => self.handle_initial_credential(cred),
            Err(err) => tracing::warn!("Restore credential: {err}"),
        }
        Ok(())
    }

    async fn restore_credential(&self) -> Result<Verified<Credential>, CredentialError> {
        let restore = auth::Restore {
            jwt_service: self.jwt_service(),
            cache: &self.cache,
            now: self.now(),
            persist_when_refreshed: true,
        };
        restore.restore().await
    }

    fn handle_initial_credential(&mut self, cred: Verified<Credential>) {
        self.set_credential(cred);
        self.initial_fetch();
        self.check_latest_release();
        self.components.auth.authenticated();
        self.reset_idle_timer();
        self.should_render();
        self.keymaps()
            .disable(KeymapId::Login)
            .enable(KeymapId::Tabs)
            .enable(KeymapId::Entries)
            .enable(KeymapId::Filter);
        self.config
            .features
            .enable_github_notification
            .then(|| self.keymaps().enable(KeymapId::Notification));
    }

    fn set_credential(&mut self, cred: Verified<Credential>) {
        self.schedule_credential_refreshing(&cred);
        self.client.set_credential(cred);
    }

    fn initial_fetch(&mut self) {
        tracing::info!("Initial fetch");
        self.jobs.futures.push(
            future::ready(Ok(Command::FetchEntries {
                after: None,
                first: self.config.entries_per_pagination,
            }))
            .boxed(),
        );
        if self.config.features.enable_github_notification {
            self.jobs.futures.push(
                future::ready(Ok(Command::FetchGhNotifications {
                    page: config::github::INITIAL_PAGE_NUM,
                    populate: Populate::Replace,
                }))
                .boxed(),
            );
        }
    }

    /// Restore terminal state and print something to console if necesseary
    fn cleanup(&mut self) -> anyhow::Result<()> {
        self.terminal.restore()?;

        // Make sure inform after terminal restored
        self.inform_latest_release();
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
        let mut queue = VecDeque::with_capacity(2);

        loop {
            let command = tokio::select! {
                biased;

                Some(event) = input.next() => {
                    self.handle_terminal_event(event)
                }
                Some(command) = self.jobs.futures.next() => {
                    Some(command.unwrap())
                }
                Some(command) = self.jobs.scheduled.next() => {
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
                queue.push_back(command);
                self.apply(&mut queue);
            }

            if self.state.flags.contains(Should::Render) {
                self.render();
                self.state.flags.remove(Should::Render);
                self.components.prompt.clear_error_message();
            }

            if self.state.flags.contains(Should::Quit) {
                self.state.flags.remove(Should::Quit); // for testing
                break ControlFlow::Break(());
            }
        }
    }

    #[tracing::instrument(skip_all)]
    fn apply(&mut self, queue: &mut VecDeque<Command>) {
        while let Some(command) = queue.pop_front() {
            let _guard = tracing::info_span!("apply", %command).entered();

            match command {
                Command::Nop => {}
                Command::Quit => self.state.flags.insert(Should::Quit),
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
                    self.init_device_flow(provider);
                }
                Command::MoveAuthenticationProvider(direction) => {
                    self.components.auth.move_selection(direction);
                    self.should_render();
                }
                Command::HandleApiResponse {
                    request_seq,
                    response,
                } => {
                    self.in_flight.remove(request_seq);

                    match response {
                        ApiResponse::DeviceFlowAuthorization {
                            provider,
                            device_authorization,
                        } => {
                            self.handle_device_flow_authorization_response(
                                provider,
                                device_authorization,
                            );
                        }
                        ApiResponse::DeviceFlowCredential { credential } => {
                            self.complete_device_authroize_flow(credential);
                        }
                        ApiResponse::SubscribeFeed { feed } => {
                            self.components.subscription.upsert_subscribed_feed(*feed);
                            self.fetch_entries(
                                Populate::Replace,
                                None,
                                self.config.entries_per_pagination,
                            );
                            self.should_render();
                        }
                        ApiResponse::UnsubscribeFeed { url } => {
                            self.components.subscription.remove_unsubscribed_feed(&url);
                            self.components.entries.remove_unsubscribed_entries(&url);
                            self.components.filter.update_categories(
                                &self.categories,
                                Populate::Replace,
                                self.components.entries.entries(),
                            );
                            self.should_render();
                        }
                        ApiResponse::FetchSubscription {
                            populate,
                            subscription,
                        } => {
                            // paginate
                            subscription.feeds.page_info.has_next_page.then(|| {
                                queue.push_back(Command::FetchSubscription {
                                    after: subscription.feeds.page_info.end_cursor.clone(),
                                    first: subscription.feeds.nodes.len().try_into().unwrap_or(0),
                                });
                            });
                            // how we show fetched errors in ui?
                            if !subscription.feeds.errors.is_empty() {
                                tracing::warn!(
                                    "Failed fetched feeds: {:?}",
                                    subscription.feeds.errors
                                );
                            }
                            self.components
                                .subscription
                                .update_subscription(populate, subscription);
                            self.should_render();
                        }
                        ApiResponse::FetchEntries { populate, payload } => {
                            self.components.filter.update_categories(
                                &self.categories,
                                populate,
                                payload.entries.as_slice(),
                            );
                            // paginate
                            payload.page_info.has_next_page.then(|| {
                                queue.push_back(Command::FetchEntries {
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
                            });
                            self.components.entries.update_entries(populate, payload);
                            self.should_render();
                        }
                        ApiResponse::FetchGithubNotifications {
                            notifications,
                            populate,
                        } => {
                            self.components
                                .gh_notifications
                                .update_notifications(populate, notifications)
                                .into_iter()
                                .for_each(|command| queue.push_back(command));
                            self.components
                                .gh_notifications
                                .fetch_next_if_needed()
                                .into_iter()
                                .for_each(|command| queue.push_back(command));
                            if populate == Populate::Replace {
                                self.components.filter.clear_gh_notifications_categories();
                            }
                            self.should_render();
                        }
                        ApiResponse::FetchGithubIssue {
                            notification_id,
                            issue,
                        } => {
                            if let Some(notification) = self
                                .components
                                .gh_notifications
                                .update_issue(notification_id, issue, &self.categories)
                            {
                                let categories = notification.categories().cloned();
                                self.components.filter.update_gh_notification_categories(
                                    &self.categories,
                                    Populate::Append,
                                    categories,
                                );
                            }
                            self.should_render();
                        }
                        ApiResponse::FetchGithubPullRequest {
                            notification_id,
                            pull_request,
                        } => {
                            if let Some(notification) =
                                self.components.gh_notifications.update_pull_request(
                                    notification_id,
                                    pull_request,
                                    &self.categories,
                                )
                            {
                                let categories = notification.categories().cloned();
                                self.components.filter.update_gh_notification_categories(
                                    &self.categories,
                                    Populate::Append,
                                    categories,
                                );
                            }
                            self.should_render();
                        }
                        ApiResponse::MarkGithubNotificationAsDone { notification_id } => {
                            self.components
                                .gh_notifications
                                .marked_as_done(notification_id);
                            self.should_render();
                        }
                        ApiResponse::UnsubscribeGithubThread { .. } => {
                            // do nothing
                        }
                    }
                }
                Command::RefreshCredential { credential } => {
                    self.set_credential(credential);
                }
                Command::MoveTabSelection(direction) => {
                    self.keymaps()
                        .disable(KeymapId::Subscription)
                        .disable(KeymapId::Entries)
                        .disable(KeymapId::Notification);

                    match self.components.tabs.move_selection(direction) {
                        Tab::Feeds => {
                            self.keymaps().enable(KeymapId::Subscription);
                            if !self.components.subscription.has_subscription() {
                                queue.push_back(Command::FetchSubscription {
                                    after: None,
                                    first: self.config.feeds_per_pagination,
                                });
                            }
                        }
                        Tab::Entries => {
                            self.keymaps().enable(KeymapId::Entries);
                        }
                        Tab::GitHub => {
                            self.keymaps().enable(KeymapId::Notification);
                        }
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
                    queue.push_back(Command::CancelFeedUnsubscriptionPopup);
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
                Command::ReloadSubscription => {
                    self.fetch_subscription(
                        Populate::Replace,
                        None,
                        self.config.feeds_per_pagination,
                    );
                    self.should_render();
                }
                Command::OpenFeed => {
                    self.open_feed();
                }
                Command::FetchEntries { after, first } => {
                    self.fetch_entries(Populate::Append, after, first);
                }
                Command::ReloadEntries => {
                    self.fetch_entries(Populate::Replace, None, self.config.entries_per_pagination);
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
                    let filterer = self.components.filter.move_requirement(direction);
                    self.apply_filterer(filterer);
                    self.should_render();
                }
                Command::ActivateCategoryFilterling => {
                    let keymap = self
                        .components
                        .filter
                        .activate_category_filtering(self.components.tabs.current().into());
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
                        let filterer = self
                            .components
                            .filter
                            .filterer(self.components.tabs.current().into());
                        self.apply_filterer(filterer);
                        self.should_render();
                    }
                }
                Command::DeactivateFiltering => {
                    self.components.filter.deactivate_filtering();
                    self.keymaps().disable(KeymapId::CategoryFiltering);
                    self.key_handlers.remove_prompt();
                    self.should_render();
                }
                Command::ToggleFilterCategory { category, lane } => {
                    let filter = self
                        .components
                        .filter
                        .toggle_category_state(&category, lane);
                    self.apply_filterer(filter)
                        .into_iter()
                        .for_each(|command| queue.push_back(command));
                    self.should_render();
                }
                Command::ActivateAllFilterCategories { lane } => {
                    let filterer = self.components.filter.activate_all_categories_state(lane);
                    self.apply_filterer(filterer);
                    self.should_render();
                }
                Command::DeactivateAllFilterCategories { lane } => {
                    let filterer = self.components.filter.deactivate_all_categories_state(lane);
                    self.apply_filterer(filterer);
                    self.should_render();
                }
                Command::FetchGhNotifications { page, populate } => {
                    self.fetch_gh_notifications(populate, page);
                }
                Command::MoveGhNotification(direction) => {
                    self.components.gh_notifications.move_selection(direction);
                    self.should_render();
                }
                Command::MoveGhNotificationFirst => {
                    self.components.gh_notifications.move_first();
                    self.should_render();
                }
                Command::MoveGhNotificationLast => {
                    self.components.gh_notifications.move_last();
                    self.should_render();
                }
                Command::OpenGhNotification => {
                    self.open_notification();
                }
                Command::ReloadGhNotifications => {
                    self.fetch_gh_notifications(
                        Populate::Replace,
                        config::github::INITIAL_PAGE_NUM,
                    );
                }
                Command::FetchGhNotificationDetails { contexts } => {
                    self.fetch_gh_notification_details(contexts);
                }
                Command::MarkGhNotificationAsDone => {
                    self.mark_gh_notification_as_done();
                }
                Command::UnsubscribeGhThread => {
                    // Unlike the web UI, simply unsubscribing does not mark it as done
                    // and it remains as unread.
                    // Therefore, when reloading, the unsubscribed notification is displayed again.
                    // To address this, we will implicitly mark it as done when unsubscribing.
                    self.unsubscribe_gh_thread();
                    self.mark_gh_notification_as_done();
                }
                Command::RotateTheme => {
                    self.rotate_theme();
                    self.should_render();
                }
                Command::InformLatestRelease(version) => {
                    self.latest_release = Some(version);
                }
                Command::HandleError { message } => {
                    self.handle_error_message(message, None);
                }
                Command::HandleApiError { error, request_seq } => {
                    let message = match Arc::into_inner(error).expect("error never cloned") {
                        SyndApiError::Unauthorized { url } => {
                            tracing::warn!(
                                "api return unauthorized status code. the cached credential are likely invalid, so try to clean cache"
                            );
                            self.cache.clean().ok();

                            format!(
                                "{} unauthorized. please login again",
                                url.map(|url| url.to_string()).unwrap_or_default(),
                            )
                        }
                        SyndApiError::BuildRequest(err) => {
                            format!("build request failed: {err} this is a BUG")
                        }

                        SyndApiError::Graphql { errors } => {
                            errors.into_iter().map(|err| err.to_string()).join(", ")
                        }
                        SyndApiError::SubscribeFeed(err) => err.to_string(),

                        SyndApiError::Internal(err) => format!("internal error: {err}"),
                    };
                    self.handle_error_message(message, Some(request_seq));
                }
                Command::HandleOauthApiError { error, request_seq } => {
                    self.handle_error_message(error.to_string(), Some(request_seq));
                }
                Command::HandleGithubApiError { error, request_seq } => {
                    self.handle_error_message(error.to_string(), Some(request_seq));
                }
            }
        }
    }

    fn handle_error_message(
        &mut self,
        error_message: String,
        request_seq: Option<RequestSequence>,
    ) {
        tracing::error!("{error_message}");

        if let Some(request_seq) = request_seq {
            self.in_flight.remove(request_seq);
        }

        self.components.prompt.set_error_message(error_message);
        self.should_render();
    }

    #[inline]
    fn should_render(&mut self) {
        self.state.flags.insert(Should::Render);
    }

    fn render(&mut self) {
        let cx = ui::Context {
            theme: &self.theme,
            in_flight: &self.in_flight,
            categories: &self.categories,
            focus: self.state.focus(),
            now: self.now(),
            tab: self.components.tabs.current(),
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
            CrosstermEvent::FocusGained => {
                self.should_render();
                self.state.focus_gained()
            }
            CrosstermEvent::FocusLost => {
                self.should_render();
                self.state.focus_lost()
            }
            CrosstermEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            }) => None,
            CrosstermEvent::Key(key) => {
                tracing::debug!("Handle key event: {key:?}");

                self.reset_idle_timer();

                match self.key_handlers.handle(key) {
                    KeyEventResult::Consumed {
                        command,
                        should_render,
                    } => {
                        should_render.then(|| self.should_render());
                        command
                    }
                    KeyEventResult::Ignored => None,
                }
            }
            _ => None,
        }
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
                    future::ready(Ok(Command::HandleError { message })).boxed()
                } else {
                    future::ready(Ok(Command::SubscribeFeed { input })).boxed()
                }
            }

            Err(err) => async move {
                Ok(Command::HandleError {
                    message: err.to_string(),
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
                Ok(feed) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::SubscribeFeed {
                        feed: Box::new(feed),
                    },
                }),
                Err(error) => Ok(Command::api_error(error, request_seq)),
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
                Ok(()) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::UnsubscribeFeed { url },
                }),
                Err(err) => Ok(Command::api_error(err, request_seq)),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    fn mark_gh_notification_as_done(&mut self) {
        let Some(id) = self.components.gh_notifications.marking_as_done() else {
            return;
        };
        let client = self.github_client.as_ref().unwrap().clone();
        let request_seq = self.in_flight.add(RequestId::MarkGithubNotificationAsDone);
        let fut = async move {
            match client.mark_thread_as_done(id).await {
                Ok(()) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::MarkGithubNotificationAsDone {
                        notification_id: id,
                    },
                }),
                Err(error) => Ok(Command::HandleGithubApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    fn unsubscribe_gh_thread(&mut self) {
        let Some(id) = self
            .components
            .gh_notifications
            .selected_notification()
            .and_then(|n| n.thread_id)
        else {
            return;
        };
        let client = self.github_client.as_ref().unwrap().clone();
        let request_seq = self.in_flight.add(RequestId::UnsubscribeGithubThread);
        let fut = async move {
            match client.unsubscribe_thread(id).await {
                Ok(()) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::UnsubscribeGithubThread {},
                }),
                Err(error) => Ok(Command::HandleGithubApiError {
                    error: Arc::new(error),
                    request_seq,
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

    fn open_notification(&mut self) {
        let Some(notification_url) = self
            .components
            .gh_notifications
            .selected_notification()
            .and_then(Notification::browser_url)
        else {
            return;
        };
        self.interactor.open_browser(notification_url.as_str());
    }
}

impl Application {
    #[tracing::instrument(skip(self))]
    fn fetch_subscription(&mut self, populate: Populate, after: Option<String>, first: i64) {
        if first <= 0 {
            return;
        }
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchSubscription);
        let fut = async move {
            match client.fetch_subscription(after, Some(first)).await {
                Ok(subscription) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::FetchSubscription {
                        populate,
                        subscription,
                    },
                }),
                Err(err) => Ok(Command::api_error(err, request_seq)),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    #[tracing::instrument(skip(self))]
    fn fetch_entries(&mut self, populate: Populate, after: Option<String>, first: i64) {
        if first <= 0 {
            return;
        }
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchEntries);
        let fut = async move {
            match client.fetch_entries(after, first).await {
                Ok(payload) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::FetchEntries { populate, payload },
                }),
                Err(error) => Ok(Command::HandleApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    #[tracing::instrument(skip(self))]
    fn fetch_gh_notifications(&mut self, populate: Populate, page: u8) {
        let client = self
            .github_client
            .clone()
            .expect("Github client not found, this is a BUG");
        let request_seq = self
            .in_flight
            .add(RequestId::FetchGithubNotifications { page });
        let fut = async move {
            match client
                .fetch_notifications(FetchNotificationsParams {
                    page,
                    include: FetchNotificationInclude::OnlyUnread,
                })
                .await
            {
                Ok(notifications) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::FetchGithubNotifications {
                        populate,
                        notifications,
                    },
                }),
                Err(error) => Ok(Command::HandleGithubApiError {
                    error: Arc::new(error),
                    request_seq,
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    #[tracing::instrument(skip(self))]
    fn fetch_gh_notification_details(&mut self, contexts: Vec<IssueOrPullRequest>) {
        let client = self
            .github_client
            .clone()
            .expect("Github client not found, this is a BUG");

        for context in contexts {
            let request_seq = self.in_flight.add(RequestId::FetchGithubSubject);
            let client = client.clone();

            let fut = match context {
                Either::Left(issue) => {
                    let notification_id = issue.notification_id;
                    async move {
                        match client.fetch_issue(issue).await {
                            Ok(issue) => Ok(Command::HandleApiResponse {
                                request_seq,
                                response: ApiResponse::FetchGithubIssue {
                                    notification_id,
                                    issue,
                                },
                            }),
                            Err(error) => Ok(Command::HandleGithubApiError {
                                error: Arc::new(error),
                                request_seq,
                            }),
                        }
                    }
                    .boxed()
                }
                Either::Right(pull_request) => {
                    let notification_id = pull_request.notification_id;
                    async move {
                        match client.fetch_pull_request(pull_request).await {
                            Ok(pull_request) => Ok(Command::HandleApiResponse {
                                request_seq,
                                response: ApiResponse::FetchGithubPullRequest {
                                    notification_id,
                                    pull_request,
                                },
                            }),
                            Err(error) => Ok(Command::HandleGithubApiError {
                                error: Arc::new(error),
                                request_seq,
                            }),
                        }
                    }
                    .boxed()
                }
            };
            self.jobs.futures.push(fut);
        }
    }
}

impl Application {
    #[tracing::instrument(skip(self))]
    fn init_device_flow(&mut self, provider: AuthenticationProvider) {
        tracing::info!("Start authenticate");

        let authenticator = self.authenticator.clone();
        let request_seq = self.in_flight.add(RequestId::DeviceFlowDeviceAuthorize);
        let fut = async move {
            match authenticator.init_device_flow(provider).await {
                Ok(device_authorization) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::DeviceFlowAuthorization {
                        provider,
                        device_authorization,
                    },
                }),
                Err(err) => Ok(Command::oauth_api_error(err, request_seq)),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    fn handle_device_flow_authorization_response(
        &mut self,
        provider: AuthenticationProvider,
        device_authorization: DeviceAuthorizationResponse,
    ) {
        self.components
            .auth
            .set_device_authorization_response(device_authorization.clone());
        self.should_render();
        // try to open input screen in the browser
        self.interactor
            .open_browser(device_authorization.verification_uri().to_string());

        let authenticator = self.authenticator.clone();
        let now = self.now();
        let request_seq = self.in_flight.add(RequestId::DeviceFlowPollAccessToken);
        let fut = async move {
            match authenticator
                .poll_device_flow_access_token(now, provider, device_authorization)
                .await
            {
                Ok(credential) => Ok(Command::HandleApiResponse {
                    request_seq,
                    response: ApiResponse::DeviceFlowCredential { credential },
                }),
                Err(err) => Ok(Command::oauth_api_error(err, request_seq)),
            }
        }
        .boxed();

        self.jobs.futures.push(fut);
    }

    fn complete_device_authroize_flow(&mut self, cred: Verified<Credential>) {
        if let Err(err) = self.cache.persist_credential(&cred) {
            tracing::error!("Failed to save credential to cache: {err}");
        }

        self.handle_initial_credential(cred);
    }

    fn schedule_credential_refreshing(&mut self, cred: &Verified<Credential>) {
        match &**cred {
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
                        }),
                    }
                }
                .boxed();
                self.jobs.scheduled.push(fut);
            }
        }
    }
}

impl Application {
    fn apply_filterer(&mut self, filterer: Filterer) -> Option<Command> {
        match filterer {
            Filterer::Feed(filterer) => {
                self.components.entries.update_filterer(filterer.clone());
                self.components.subscription.update_filterer(filterer);
                None
            }
            Filterer::GhNotification(filterer) => {
                self.components.gh_notifications.update_filterer(filterer);
                self.components.gh_notifications.fetch_next_if_needed()
            }
        }
    }

    fn rotate_theme(&mut self) {
        let p = match self.theme.name {
            "ferra" => Palette::solarized_dark(),
            "solarized_dark" => Palette::helix(),
            _ => Palette::ferra(),
        };
        self.theme = Theme::with_palette(&p);
    }
}

impl Application {
    fn check_latest_release(&mut self) {
        use update_informer::{registry, Check};

        // update informer use reqwest::blocking
        let check = tokio::task::spawn_blocking(|| {
            let name = env!("CARGO_PKG_NAME");
            let version = env!("CARGO_PKG_VERSION");
            #[cfg(not(test))]
            let informer = update_informer::new(registry::Crates, name, version)
                .interval(Duration::from_secs(60 * 60 * 24))
                .timeout(Duration::from_secs(5));

            #[cfg(test)]
            let informer = update_informer::fake(registry::Crates, name, version, "v1.0.0");

            informer.check_version().ok().flatten()
        });
        let fut = async move {
            match check.await {
                Ok(Some(version)) => Ok(Command::InformLatestRelease(version)),
                _ => Ok(Command::Nop),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    fn inform_latest_release(&self) {
        let current_version = env!("CARGO_PKG_VERSION");
        if let Some(new_version) = &self.latest_release {
            println!("A new release of synd is available: v{current_version} -> {new_version}");
        }
    }
}

impl Application {
    fn handle_idle(&mut self) {
        self.clear_idle_timer();

        #[cfg(feature = "integration")]
        {
            tracing::debug!("Quit for idle");
            self.state.flags.insert(Should::Quit);
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
    pub fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.terminal.buffer()
    }

    pub async fn wait_until_jobs_completed<S>(&mut self, input: &mut S)
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        loop {
            self.event_loop_until_idle(input).await;
            self.reset_idle_timer();

            // In the current test implementation, we synchronie
            // the assertion timing by waiting until jobs are empty.
            // However the future of refreshing the id token sleeps until it expires and remains in the jobs for long time
            // Therefore, we ignore scheduled jobs
            if self.jobs.futures.is_empty() {
                break;
            }
        }
    }

    pub async fn reload_cache(&mut self) -> anyhow::Result<()> {
        match self.restore_credential().await {
            Ok(cred) => self.handle_initial_credential(cred),
            Err(err) => return Err(err.into()),
        }
        Ok(())
    }
}
