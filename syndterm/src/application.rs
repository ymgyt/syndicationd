use crate::{
    auth::{
        self,
        device_flow::{DeviceAccessTokenResponse, DeviceAuthorizationResponse},
        Authentication,
    },
    client::{query::user::UserSubscription, Client},
    command::Command,
    job::Jobs,
    terminal::Terminal,
    ui::{self, login::LoginMethods},
};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind};
use futures_util::{FutureExt, Stream, StreamExt};
use tracing::info;

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
    pub user_subscription: Option<UserSubscription>,
}

pub struct Application {
    terminal: Terminal,
    client: Client,
    jobs: Jobs,
    state: State,
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
            user_subscription: None,
        };

        Self {
            terminal,
            client,
            jobs: Jobs::new(),
            state,
            should_quit: false,
            should_render: false,
        }
    }

    pub fn set_auth(&mut self, auth: Authentication) {
        self.client.set_credential(auth);
        self.state.login.auth_state = AuthenticateState::Authenticated;
        self.state.screen = Screen::Browse;
        self.should_render = true;
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
            }

            if self.should_quit {
                break;
            }
        }
    }

    fn apply(&mut self, command: Command) {
        info!("Apply {command:?}");
        match command {
            Command::Quit => self.should_quit = true,
            Command::Authenticate(method) => self.authenticate(method),
            Command::DeviceAuthorizationFlow(device_authorization) => {
                self.device_authorize_flow(device_authorization)
            }
            Command::CompleteDevieAuthorizationFlow(device_access_token) => {
                self.complete_device_authroize_flow(device_access_token)
            }
            Command::FetchSubscription => self.fetch_subscription(),
            Command::UpdateSubscription(sub) => {
                self.state.user_subscription = Some(sub);
                self.should_render = true;
            }
        }
    }

    fn render(&mut self) {
        let cx = ui::Context {
            state: &mut self.state,
        };

        self.terminal.render(|frame| ui::render(frame, cx)).unwrap();
    }

    fn handle_terminal_event(&mut self, event: std::io::Result<CrosstermEvent>) -> Option<Command> {
        match event.unwrap() {
            CrosstermEvent::Resize(_, _) => None,
            // Ignore release events
            CrosstermEvent::Key(KeyEvent {
                kind: KeyEventKind::Release,
                ..
            }) => None,
            CrosstermEvent::Key(key) => {
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
                        KeyCode::Char('r') => return Some(Command::FetchSubscription),
                        _ => {}
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
    fn fetch_subscription(&mut self) {
        let client = self.client.clone();
        let fut = async move {
            let sub = client.fetch_subscription().await.unwrap();
            Ok(Command::UpdateSubscription(sub))
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
