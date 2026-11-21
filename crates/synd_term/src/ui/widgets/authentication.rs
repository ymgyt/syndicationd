use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget,
        Widget,
    },
};
use tui_widgets::big_text::{BigText, PixelSize};

use crate::{
    auth::AuthenticationProvider,
    ui::{self, extension::RectExt, icon, theme::Theme},
};

pub(crate) fn render_login(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    providers: &[AuthenticationProvider],
    selected_provider: usize,
) {
    let area = RectExt::centered(area, 40, 50);
    let [big_text_area, title_area, methods_area] = Layout::vertical([
        Constraint::Length(9),
        Constraint::Length(2),
        Constraint::Min(2),
    ])
    .areas(area);

    BigText::builder()
        .pixel_size(PixelSize::HalfWidth)
        .style(theme.base)
        .alignment(Alignment::Center)
        .lines(vec!["Syndicationd".into()])
        .build()
        .render(big_text_area, buf);

    let methods = providers
        .iter()
        .map(|provider| match provider {
            AuthenticationProvider::Gh => Text::from(concat!(icon!(gh), " GitHub")),
            AuthenticationProvider::Google => Text::from(concat!(icon!(google), " Google")),
        })
        .map(ListItem::new);
    let methods = List::new(methods)
        .highlight_symbol(ui::TABLE_HIGHLIGHT_SYMBOL)
        .highlight_style(theme.login.selected_auth_provider_item)
        .highlight_spacing(HighlightSpacing::Always);
    let mut methods_state = ListState::default().with_selected(Some(selected_provider));

    Widget::render(login_title(theme), title_area, buf);
    StatefulWidget::render(methods, methods_area, buf, &mut methods_state);
}

pub(crate) fn render_device_flow(
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    verification_url: &str,
    user_code: &str,
) {
    let area = RectExt::centered(area, 40, 50);
    let [title_area, device_flow_area] =
        Layout::vertical([Constraint::Length(2), Constraint::Min(1)]).areas(area);
    let device_flow = Paragraph::new(vec![
        Line::from("Open the following URL and Enter the code"),
        Line::from(""),
        Line::from(vec![
            Span::styled("URL:  ", Style::default()),
            Span::styled(
                verification_url,
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Code: ", Style::default()),
            Span::styled(user_code, Style::default().add_modifier(Modifier::BOLD)),
        ]),
    ]);

    Widget::render(login_title(theme), title_area, buf);
    Widget::render(device_flow, device_flow_area, buf);
}

fn login_title(theme: &Theme) -> Paragraph<'static> {
    Paragraph::new(Span::styled("Login", theme.login.title))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::BOTTOM))
}
