use std::{pin::Pin, time::Duration};

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind};
use futures_util::{FutureExt, Stream, StreamExt};
use ratatui::{style::palette::tailwind, widgets::Widget};
use synd_authn::device_flow::{
    github::DeviceFlow, DeviceAccessTokenResponse, DeviceAuthorizationResponse,
};
use tokio::time::{Instant, Sleep};

use crate::{
    auth::{AuthenticationProvider, Credential},
    client::Client,
    command::Command,
    config,
    interact::Interactor,
    job::Jobs,
    terminal::Terminal,
    ui::{
        self,
        components::{authentication::AuthenticateState, root::Root, tabs::Tab, Components},
        theme::Theme,
    },
};

mod direction;
pub use direction::{Direction, IndexOutOfRange};

mod in_flight;
pub use in_flight::{InFlight, RequestId, RequestSequence};

enum Screen {
    Login,
    Browse,
}

#[derive(PartialEq, Eq)]
pub enum EventLoopControlFlow {
    Quit,
}

pub struct Config {
    pub idle_timer_interval: Duration,
    pub throbber_timer_interval: Duration,
    pub github_device_flow: DeviceFlow,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            idle_timer_interval: Duration::from_secs(250),
            throbber_timer_interval: Duration::from_millis(250),
            github_device_flow: DeviceFlow::new(config::github::CLIENT_ID),
        }
    }
}

pub struct Application {
    terminal: Terminal,
    client: Client,
    jobs: Jobs,
    components: Components,
    interactor: Interactor,
    in_flight: InFlight,
    theme: Theme,
    idle_timer: Pin<Box<Sleep>>,
    config: Config,

    screen: Screen,
    should_render: bool,
    should_quit: bool,
}

impl Application {
    pub fn new(terminal: Terminal, client: Client) -> Self {
        Application::with(terminal, client, Config::default())
    }

    pub fn with(terminal: Terminal, client: Client, config: Config) -> Self {
        Self {
            terminal,
            client,
            jobs: Jobs::new(),
            components: Components::new(),
            interactor: Interactor::new(),
            in_flight: InFlight::new().with_throbber_timer_interval(config.throbber_timer_interval),
            theme: Theme::with_palette(&tailwind::BLUE),
            idle_timer: Box::pin(tokio::time::sleep(config.idle_timer_interval)),
            screen: Screen::Login,
            config,
            should_quit: false,
            should_render: false,
        }
    }

    #[must_use]
    pub fn with_theme(self, theme: Theme) -> Self {
        Self { theme, ..self }
    }

    pub fn set_credential(&mut self, cred: Credential) {
        self.client.set_credential(cred);
        self.components.auth.authenticated();
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
                first: 200,
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

        self.terminal.exit()?;

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

        // how to handle command => command pattern ?
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
                Command::Authenticate(method) => self.authenticate(method),
                Command::DeviceAuthorizationFlow(device_authorization) => {
                    self.device_authorize_flow(device_authorization);
                }
                Command::CompleteDevieAuthorizationFlow(device_access_token) => {
                    self.complete_device_authroize_flow(device_access_token);
                }
                Command::MoveTabSelection(direction) => {
                    match self.components.tabs.move_selection(&direction) {
                        Tab::Feeds if !self.components.subscription.has_subscription() => {
                            next = Some(Command::FetchSubscription {
                                after: None,
                                first: 50,
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
                Command::PromptFeedSubscription => {
                    self.prompt_feed_subscription();
                    self.should_render = true;
                }
                Command::PromptFeedUnsubscription => {
                    self.prompt_feed_unsubscription();
                    self.should_render = true;
                }
                Command::SubscribeFeed { url } => {
                    self.subscribe_feed(url);
                    self.should_render = true;
                }
                Command::UnsubscribeFeed { url } => {
                    self.unsubscribe_feed(url);
                    self.should_render = true;
                }
                Command::FetchSubscription { after, first } => {
                    self.fetch_subscription(after, first);
                }
                Command::UpdateSubscription {
                    subscription,
                    request_seq,
                } => {
                    self.in_flight.remove(request_seq);
                    self.components
                        .subscription
                        .update_subscription(subscription);
                    self.should_render = true;
                }
                Command::CompleteSubscribeFeed { feed } => {
                    self.components.subscription.add_subscribed_feed(feed);
                    self.should_render = true;
                }
                Command::CompleteUnsubscribeFeed { url } => {
                    self.components.subscription.remove_unsubscribed_feed(&url);
                    self.should_render = true;
                }
                Command::OpenFeed => {
                    self.open_feed();
                }
                Command::FetchEntries { after, first } => {
                    self.fetch_entries(after, first);
                }
                Command::UpdateEntries {
                    payload,
                    request_seq,
                } => {
                    self.in_flight.remove(request_seq);
                    self.components.entries.update_entries(payload);
                    self.should_render = true;
                }
                Command::MoveEntry(direction) => {
                    self.components.entries.move_selection(&direction);
                    self.should_render = true;
                }
                Command::OpenEntry => {
                    self.open_entry();
                }
                Command::HandleError {
                    message,
                    request_seq,
                } => {
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
        };
        let root = Root::new(&self.components, cx);

        self.terminal
            .render(|frame| Widget::render(root, frame.size(), frame.buffer_mut()))
            .expect("Failed to render");
    }

    #[allow(clippy::single_match)]
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
                self.reset_idle_timer();

                tracing::debug!("Handle key event: {key:?}");
                match self.screen {
                    Screen::Login => match key.code {
                        KeyCode::Enter => {
                            if self.components.auth.state() == &AuthenticateState::NotAuthenticated
                            {
                                return Some(Command::Authenticate(
                                    self.components.auth.selected_provider(),
                                ));
                            };
                        }
                        _ => {}
                    },
                    Screen::Browse => match key.code {
                        KeyCode::Tab => return Some(Command::MoveTabSelection(Direction::Right)),
                        KeyCode::BackTab => {
                            return Some(Command::MoveTabSelection(Direction::Left))
                        }
                        _ => match self.components.tabs.current() {
                            Tab::Entries => match key.code {
                                KeyCode::Char('j') => {
                                    return Some(Command::MoveEntry(Direction::Down))
                                }
                                KeyCode::Char('k') => {
                                    return Some(Command::MoveEntry(Direction::Up))
                                }
                                KeyCode::Enter => return Some(Command::OpenEntry),
                                _ => {}
                            },
                            Tab::Feeds => match key.code {
                                KeyCode::Char('a') => return Some(Command::PromptFeedSubscription),
                                KeyCode::Char('d') => {
                                    return Some(Command::PromptFeedUnsubscription)
                                }
                                KeyCode::Char('j') => {
                                    return Some(Command::MoveSubscribedFeed(Direction::Down));
                                }
                                KeyCode::Char('k') => {
                                    return Some(Command::MoveSubscribedFeed(Direction::Up));
                                }
                                KeyCode::Enter => return Some(Command::OpenFeed),
                                _ => {}
                            },
                        },
                    },
                };
                match key.code {
                    KeyCode::Char('q') => Some(Command::Quit),
                    _ => None,
                }
            }
            _ => None,
        }
    }
}

impl Application {
    fn fetch_subscription(&mut self, after: Option<String>, first: i64) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchSubscription);
        let fut = async move {
            match client.fetch_subscription(after, Some(first)).await {
                Ok(subscription) => Ok(Command::UpdateSubscription {
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
        let prompt = r"URL: https://blog.ymgyt.io/atom.xml";
        let modified = edit::edit(prompt).expect("Got user modified input");
        tracing::debug!("Got user modified feed subscription: {modified}");
        // TODO: more safely
        let url = modified.trim_start_matches("URL:").trim().to_owned();

        let fut = async move { Ok(Command::SubscribeFeed { url }) }.boxed();
        self.jobs.futures.push(fut);
    }

    fn prompt_feed_unsubscription(&mut self) {
        // TODO: prompt deletion confirm
        let Some(url) = self
            .components
            .subscription
            .selected_feed_url()
            .map(ToOwned::to_owned)
        else {
            return;
        };
        let fut = async move { Ok(Command::UnsubscribeFeed { url }) }.boxed();
        self.jobs.futures.push(fut);
    }

    fn subscribe_feed(&mut self, url: String) {
        let client = self.client.clone();
        let fut = async move {
            // TODO: error handling
            let feed = client.subscribe_feed(url).await.unwrap();
            Ok(Command::CompleteSubscribeFeed { feed })
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    fn unsubscribe_feed(&mut self, url: String) {
        let client = self.client.clone();
        let fut = async move {
            // TODO: error handling
            client.unsubscribe_feed(url.clone()).await.unwrap();
            Ok(Command::CompleteUnsubscribeFeed { url })
        }
        .boxed();
        self.jobs.futures.push(fut);
    }
}

impl Application {
    fn open_feed(&mut self) {
        let Some(feed_website_url) = self.components.subscription.selected_feed_website_url()
        else {
            return;
        };
        open::that(feed_website_url).ok();
    }

    fn open_entry(&mut self) {
        let Some(entry_website_url) = self.components.entries.selected_entry_website_url() else {
            return;
        };
        open::that(entry_website_url).ok();
    }
}

impl Application {
    #[tracing::instrument(skip(self))]
    fn fetch_entries(&mut self, after: Option<String>, first: i64) {
        let client = self.client.clone();
        let request_seq = self.in_flight.add(RequestId::FetchEntries);
        let fut = async move {
            match client.fetch_entries(after, first).await {
                Ok(payload) => Ok(Command::UpdateEntries {
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
    #[tracing::instrument(skip(self))]
    fn authenticate(&mut self, provider: AuthenticationProvider) {
        tracing::info!("Start authenticate");
        match provider {
            AuthenticationProvider::Github => {
                let device_flow = self.config.github_device_flow.clone();
                let fut = async move {
                    match device_flow.device_authorize_request().await {
                        Ok(res) => Ok(Command::DeviceAuthorizationFlow(res)),
                        Err(err) => Ok(Command::HandleError {
                            message: format!("{err}"),
                            request_seq: None,
                        }),
                    }
                }
                .boxed();
                self.jobs.futures.push(fut);
            }
        }
    }

    fn device_authorize_flow(&mut self, device_authorization: DeviceAuthorizationResponse) {
        self.components
            .auth
            .set_device_authorization_response(device_authorization.clone());
        self.should_render = true;

        // try to open input screen in the browser
        self.interactor
            .open_browser(device_authorization.verification_uri.to_string());

        let device_flow = self.config.github_device_flow.clone();
        let fut = async move {
            match device_flow
                .pool_device_access_token(
                    device_authorization.device_code,
                    device_authorization.interval,
                )
                .await
            {
                Ok(res) => Ok(Command::CompleteDevieAuthorizationFlow(res)),
                Err(err) => Ok(Command::HandleError {
                    message: format!("{err}"),
                    request_seq: None,
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }

    fn complete_device_authroize_flow(&mut self, device_access_token: DeviceAccessTokenResponse) {
        let auth = Credential::Github {
            access_token: device_access_token.access_token,
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
