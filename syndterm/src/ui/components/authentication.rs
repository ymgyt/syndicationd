use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::{
    auth::{device_flow::DeviceAuthorizationResponse, AuthenticationProvider},
    ui::{extension::RectExt, Context},
};

/// Handle user authentication
#[derive(PartialEq, Eq)]
pub enum AuthenticateState {
    NotAuthenticated,
    DeviceFlow(DeviceAuthorizationResponse),
    Authenticated,
}

pub struct Authentication {
    state: AuthenticateState,
    providers: Vec<AuthenticationProvider>,
    selected_provider_index: usize,
}

impl Authentication {
    pub fn new(providers: Vec<AuthenticationProvider>) -> Self {
        debug_assert!(!providers.is_empty());

        Self {
            state: AuthenticateState::NotAuthenticated,
            providers,
            selected_provider_index: 0,
        }
    }

    pub fn authenticated(&mut self) {
        self.state = AuthenticateState::Authenticated;
    }

    pub fn selected_provider(&self) -> AuthenticationProvider {
        self.providers[self.selected_provider_index]
    }

    pub(super) fn should_render(&self) -> bool {
        match self.state {
            AuthenticateState::NotAuthenticated | AuthenticateState::DeviceFlow(_) => true,
            _ => false,
        }
    }
}

impl Authentication {
    pub(super) fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        match self.state {
            AuthenticateState::NotAuthenticated => self.render_login(area, buf),
            AuthenticateState::DeviceFlow(ref res) => self.render_device_flow(area, buf, cx, res),
            AuthenticateState::Authenticated => unreachable!(),
        }
    }

    fn render_login(&self, area: Rect, buf: &mut Buffer) {
        let area = area.centered(50, 50);

        let vertical = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]);
        let [title_area, methods_area] = area.split(&vertical);

        let title = Paragraph::new(Text::styled("Login", Style::default()))
            .block(Block::default().borders(Borders::BOTTOM));

        let methods = {
            let items = vec![ListItem::new(Line::styled("with Github", Style::default()))];

            List::new(items).highlight_symbol(">> ")
        };
        let mut methods_state = ListState::default().with_selected(Some(0));

        Widget::render(title, title_area, buf);
        StatefulWidget::render(methods, methods_area, buf, &mut methods_state);
    }

    fn render_device_flow(
        &self,
        area: Rect,
        buf: &mut Buffer,
        cx: &Context<'_>,
        res: &DeviceAuthorizationResponse,
    ) {
        let area = area.centered(50, 50);

        let vertical = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]);

        let [title_area, device_flow_area] = area.split(&vertical);

        let title = Paragraph::new(Text::styled("Login", Style::default()))
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

        Widget::render(title, title_area, buf);
        Widget::render(device_flow, device_flow_area, buf);
    }
}
