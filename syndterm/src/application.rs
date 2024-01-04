use crate::{
    client::{query::user::UserSubscription, Client},
    command::Command,
    job::Jobs,
    terminal::Terminal,
    ui,
};
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyEventKind};
use futures_util::{FutureExt, Stream, StreamExt};

pub struct State {
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
        match command {
            Command::Quit => self.should_quit = true,
            Command::FetchSubscription => self.fetch_subscription(),
            Command::UpdateSubscription(sub) => {
                self.state.user_subscription = Some(sub);
                self.should_render = true;
            }
        }
    }

    fn render(&mut self) {
        let cx = ui::Context {
            app_state: &self.state,
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
            CrosstermEvent::Key(key) => match key.code {
                KeyCode::Char('q') => Some(Command::Quit),
                KeyCode::Char('r') => Some(Command::FetchSubscription),
                _ => None,
            },
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
