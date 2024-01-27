use crate::{
    auth::{
        self,
        device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse},
        Authentication,
    },
    client::Client,
    command::Command,
    job::Jobs,
    terminal::Terminal,
    ui::{
        self,
        entries::Entries,
        login::LoginMethods,
        prompt::Prompt,
        subscription::Subscription,
        tabs::{Tab, Tabs},
        theme::Theme,
    },
};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind};
use futures_util::{FutureExt, Stream, StreamExt};

mod direction;
pub use direction::{Direction, IndexOutOfRange};

/// Cureent ui screen
pub enum Screen {
    Login,
    Browse,
}

/// Handle user authentication
#[derive(PartialEq, Eq)]
pub enum AuthenticateState {
    NotAuthenticated,
    DeviceFlow(DeviceAuthorizationResponse),
    Authenticated,
}

pub struct LoginState {
    pub login_methods: LoginMethods,
    pub auth_state: AuthenticateState,
}

pub struct State {
    pub screen: Screen,
    pub login: LoginState,
    pub tabs: Tabs,
    pub prompt: Prompt,
    pub subscription: Subscription,
    pub entries: Entries,
}

pub struct Application {
    terminal: Terminal,
    client: Client,
    jobs: Jobs,
    state: State,
    theme: Theme,
    should_render: bool,
    should_quit: bool,
}

impl Application {
    pub fn new(terminal: Terminal, client: Client) -> Self {
        let state = State {
            screen: Screen::Login,
            login: LoginState {
                login_methods: LoginMethods::new(),
                auth_state: AuthenticateState::NotAuthenticated,
            },
            tabs: Tabs::new(),
            subscription: Subscription::new(),
            entries: Entries::new(),
            prompt: Prompt::new(),
        };

        Self {
            terminal,
            client,
            jobs: Jobs::new(),
            state,
            theme: Theme::new(),
            should_quit: false,
            should_render: false,
        }
    }

    pub fn set_auth(&mut self, auth: Authentication) {
        self.client.set_credential(auth);
        self.state.login.auth_state = AuthenticateState::Authenticated;
        self.state.screen = Screen::Browse;
        self.initial_fetch();
        self.should_render = true;
    }

    fn initial_fetch(&mut self) {
        let fut = async {
            Ok(Command::FetchEntries {
                after: None,
                first: 50,
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

        self.render();

        self.event_loop(input).await;

        self.terminal.exit()?;

        Ok(())
    }

    async fn event_loop<S>(&mut self, input: &mut S)
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
            };

            if let Some(command) = command {
                self.apply(command);
            }

            if self.should_render {
                self.render();
                self.should_render = false;
                self.state.prompt.clear_error_message();
            }

            if self.should_quit {
                break;
            }
        }
    }

    fn apply(&mut self, command: Command) {
        tracing::debug!("Apply {command:?}");

        let mut next = Some(command);

        // how to handle command => command pattern ?
        // should detect infinite loop ?
        while let Some(command) = next.take() {
            match command {
                Command::Quit => self.should_quit = true,
                Command::ResizeTerminal { .. } => {
                    self.should_render = true;
                }
                Command::Authenticate(method) => self.authenticate(method),
                Command::DeviceAuthorizationFlow(device_authorization) => {
                    self.device_authorize_flow(device_authorization)
                }
                Command::CompleteDevieAuthorizationFlow(device_access_token) => {
                    self.complete_device_authroize_flow(device_access_token)
                }
                Command::MoveTabSelection(direction) => {
                    match self.state.tabs.move_selection(direction) {
                        Tab::Subscription if !self.state.subscription.has_subscription() => {
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
                    self.state.subscription.move_selection(direction);
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
                    self.fetch_subscription(after, first)
                }
                Command::UpdateSubscription(sub) => {
                    self.state.subscription.update_subscription(sub);
                    self.should_render = true;
                }
                Command::CompleteSubscribeFeed { feed } => {
                    self.state.subscription.add_subscribed_feed(feed);
                    self.should_render = true;
                }
                Command::CompleteUnsubscribeFeed { url } => {
                    self.state.subscription.remove_unsubscribed_feed(url);
                    self.should_render = true;
                }
                Command::OpenFeed => {
                    self.open_feed();
                }
                Command::FetchEntries { after, first } => {
                    self.fetch_entries(after, first);
                }
                Command::UpdateEntries(payload) => {
                    self.state.entries.update_entries(payload);
                    self.should_render = true;
                }
                Command::MoveEntry(direction) => {
                    self.state.entries.move_selection(direction);
                    self.should_render = true;
                }
                Command::OpenEntry => {
                    self.open_entry();
                }
                Command::HandleError { message } => {
                    self.state.prompt.set_error_message(message);
                    self.should_render = true;
                }
            }
        }
    }

    fn render(&mut self) {
        let cx = ui::Context {
            state: &mut self.state,
            theme: &self.theme,
        };

        self.terminal.render(|frame| ui::render(frame, cx)).unwrap();
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
                tracing::debug!("{key:?}");
                match self.state.screen {
                    Screen::Login => match key.code {
                        KeyCode::Enter => {
                            if self.state.login.auth_state == AuthenticateState::NotAuthenticated {
                                return Some(Command::Authenticate(
                                    self.state.login.login_methods.selected_method(),
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
                        _ => match self.state.tabs.current() {
                            Tab::Feeds => match key.code {
                                KeyCode::Char('j') => {
                                    return Some(Command::MoveEntry(Direction::Down))
                                }
                                KeyCode::Char('k') => {
                                    return Some(Command::MoveEntry(Direction::Up))
                                }
                                KeyCode::Enter => return Some(Command::OpenEntry),
                                _ => {}
                            },
                            Tab::Subscription => match key.code {
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
        let fut = async move {
            // TODO: handling
            let sub = client.fetch_subscription(after, Some(first)).await.unwrap();
            Ok(Command::UpdateSubscription(sub))
        }
        .boxed();
        self.jobs.futures.push(fut);
    }
}

impl Application {
    fn prompt_feed_subscription(&mut self) {
        let prompt = r#"URL: https://blog.ymgyt.io/atom.xml "#;
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
            .state
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
        let Some(feed_website_url) = self.state.subscription.selected_feed_website_url() else {
            return;
        };
        open::that(feed_website_url).ok();
    }

    fn open_entry(&mut self) {
        let Some(entry_website_url) = self.state.entries.selected_entry_website_url() else {
            return;
        };
        open::that(entry_website_url).ok();
    }
}

impl Application {
    fn fetch_entries(&mut self, after: Option<String>, first: i64) {
        let client = self.client.clone();
        let fut = async move {
            match client.fetch_entries(after, first).await {
                Ok(payload) => Ok(Command::UpdateEntries(payload)),
                Err(err) => Ok(Command::HandleError {
                    message: format!("{err}"),
                }),
            }
        }
        .boxed();
        self.jobs.futures.push(fut);
    }
}

#[derive(Debug)]
pub enum AuthenticateMethod {
    Github,
}

impl Application {
    fn authenticate(&mut self, method: AuthenticateMethod) {
        match method {
            AuthenticateMethod::Github => {
                let fut = async move {
                    // TODO: error handling
                    let res = auth::github::DeviceFlow::new()
                        .device_authorize_request()
                        .await
                        .unwrap();
                    Ok(Command::DeviceAuthorizationFlow(res))
                }
                .boxed();
                self.jobs.futures.push(fut);
            }
        }
    }

    fn device_authorize_flow(&mut self, device_authorization: DeviceAuthorizationResponse) {
        self.state.login.auth_state = AuthenticateState::DeviceFlow(device_authorization.clone());
        self.should_render = true;

        // attempt to open input screen in the browser
        open::that(device_authorization.verification_uri.to_string()).ok();

        let fut = async move {
            // TODO: error handling
            let res = auth::github::DeviceFlow::new()
                .pool_device_access_token(
                    device_authorization.device_code,
                    device_authorization.interval,
                )
                .await
                .unwrap();
            Ok(Command::CompleteDevieAuthorizationFlow(res))
        }
        .boxed();
        self.jobs.futures.push(fut);
        // open verification uri
        // prompt user to enter user_code
    }

    fn complete_device_authroize_flow(&mut self, device_access_token: DeviceAccessTokenResponse) {
        let auth = Authentication::Github {
            access_token: device_access_token.access_token,
        };

        // TODO: handle error
        auth::persist_authentication(auth.clone()).ok();

        self.set_auth(auth);
    }
}
