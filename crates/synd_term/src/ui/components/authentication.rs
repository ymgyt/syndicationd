use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget,
        Widget,
    },
};
use synd_auth::device_flow::DeviceAuthorizationResponse;
use tui_big_text::{BigText, PixelSize};

use crate::{
    application::Direction,
    auth::AuthenticationProvider,
    ui::{self, extension::RectExt, Context},
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

    pub fn state(&self) -> &AuthenticateState {
        &self.state
    }

    pub fn selected_provider(&self) -> AuthenticationProvider {
        self.providers[self.selected_provider_index]
    }

    pub fn move_selection(&mut self, direction: &Direction) {
        self.selected_provider_index = direction.apply(
            self.selected_provider_index,
            self.providers.len(),
            crate::application::IndexOutOfRange::Wrapping,
        );
    }

    pub fn authenticated(&mut self) {
        self.state = AuthenticateState::Authenticated;
    }

    pub fn set_device_authorization_response(&mut self, response: DeviceAuthorizationResponse) {
        self.state = AuthenticateState::DeviceFlow(response);
    }

    pub(super) fn should_render(&self) -> bool {
        matches!(
            self.state,
            AuthenticateState::NotAuthenticated | AuthenticateState::DeviceFlow(_)
        )
    }
}

impl Authentication {
    pub(super) fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        match self.state {
            AuthenticateState::NotAuthenticated => self.render_login(area, buf, cx),
            AuthenticateState::DeviceFlow(ref res) => Self::render_device_flow(area, buf, cx, res),
            AuthenticateState::Authenticated => unreachable!(),
        }
    }

    fn render_login(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let area = area.centered(40, 50);

        let vertical = Layout::vertical([
            Constraint::Length(9),
            Constraint::Length(2),
            Constraint::Min(2),
        ]);

        let [big_text_area, title_area, methods_area] = vertical.areas(area);

        // Render big "syndicationd"
        if let Ok(big_text) = BigText::builder()
            .pixel_size(PixelSize::HalfWidth)
            .style(Style::new().white())
            .alignment(Alignment::Center)
            .lines(vec!["Syndicationd".into()])
            .build()
        {
            big_text.render(big_text_area, buf);
        }

        let title = Self::login_title(cx);

        let methods = {
            let items = self
                .providers
                .iter()
                .map(|provider| match provider {
                    AuthenticationProvider::Github => Text::from("󰊤 GitHub"),
                    AuthenticationProvider::Google => Text::from("󰊭 Google"),
                })
                .map(ListItem::new);

            List::new(items)
                .highlight_symbol(ui::TABLE_HIGHLIGHT_SYMBOL)
                .highlight_style(cx.theme.login.selected_auth_provider_item)
                .highlight_spacing(HighlightSpacing::Always)
        };
        let mut methods_state =
            ListState::default().with_selected(Some(self.selected_provider_index));

        Widget::render(title, title_area, buf);
        StatefulWidget::render(methods, methods_area, buf, &mut methods_state);
    }

    fn render_device_flow(
        area: Rect,
        buf: &mut Buffer,
        cx: &Context<'_>,
        res: &DeviceAuthorizationResponse,
    ) {
        let area = area.centered(40, 50);

        let vertical = Layout::vertical([Constraint::Length(2), Constraint::Min(1)]);

        let [title_area, device_flow_area] = vertical.areas(area);

        let title = Self::login_title(cx);

        let device_flow = Paragraph::new(vec![
            Line::from("Open the following URL and Enter the code"),
            Line::from(""),
            Line::from(vec![
                Span::styled("URL:  ", Style::default()),
                Span::styled(
                    res.verification_uri().to_string(),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Code: ", Style::default()),
                Span::styled(
                    &res.user_code,
                    Style::default().add_modifier(Modifier::BOLD),
                ),
            ]),
        ]);

        Widget::render(title, title_area, buf);
        Widget::render(device_flow, device_flow_area, buf);
    }

    fn login_title(cx: &Context<'_>) -> Paragraph<'static> {
        Paragraph::new(Span::styled("Login", cx.theme.login.title))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::BOTTOM))
    }
}
