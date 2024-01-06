use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::{
    application::{AuthenticateMethod, AuthenticateState},
    auth::device_flow::DeviceAuthorizationResponse,
};

use super::{centered, Context};

pub struct LoginMethods {
    state: ListState,
    methods: List<'static>,
}

impl LoginMethods {
    pub fn new() -> Self {
        let methods = {
            let items = vec![ListItem::new(Line::styled("Github", Style::default()))];

            List::new(items).highlight_symbol(">> ")
        };

        Self {
            state: ListState::default().with_selected(Some(0)),
            methods,
        }
    }

    pub fn selected_method(&self) -> AuthenticateMethod {
        match self.state.selected() {
            Some(0) => AuthenticateMethod::Github,
            _ => unreachable!(),
        }
    }

    pub fn render(&mut self, rect: Rect, frame: &mut Frame) {
        let rect = centered(50, 50, rect);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(rect);

        let title = Paragraph::new(Text::styled("Login", Style::default().fg(Color::Green)))
            .block(Block::default().borders(Borders::BOTTOM));

        frame.render_widget(title, chunks[0]);
        frame.render_stateful_widget(self.methods.clone(), chunks[1], &mut self.state)
    }
}

fn render_device_flow(rect: Rect, frame: &mut Frame, res: &DeviceAuthorizationResponse) {
    let rect = centered(50, 50, rect);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(2), Constraint::Min(1)])
        .split(rect);

    let title = Paragraph::new(Text::styled("Login", Style::default().fg(Color::Green)))
        .block(Block::default().borders(Borders::BOTTOM));

    let device_flow = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Code: ", Style::default()),
            Span::styled(
                &res.user_code,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("URL: ", Style::default()),
            Span::styled(
                res.verification_uri.to_string(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
    ]);

    frame.render_widget(title, chunks[0]);
    frame.render_widget(device_flow, chunks[1]);
}

pub fn render(rect: Rect, frame: &mut Frame, cx: &mut Context<'_>) {
    match cx.state.login.auth_state {
        AuthenticateState::NotAuthenticated => cx.state.login.login_methods.render(rect, frame),
        AuthenticateState::DeviceFlow(ref res) => render_device_flow(rect, frame, res),
        AuthenticateState::Authenticated => unreachable!(),
    }
}
