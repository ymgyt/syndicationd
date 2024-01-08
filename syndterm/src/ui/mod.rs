use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Widget},
    Frame,
};

use crate::{
    application::{Screen, State},
    ui::theme::Theme,
};

pub mod login;
pub mod tabs;
pub mod theme;

pub struct Context<'a> {
    pub state: &'a mut State,
    pub theme: &'a Theme,
}

pub fn render(frame: &mut Frame, mut cx: Context<'_>) {
    match cx.state.screen {
        Screen::Login => return login::render(frame.size(), frame, &mut cx),
        Screen::Browse => {}
    }
    let area = frame.size();
    // Background
    Block::new()
        .style(cx.theme.background)
        .render(area, frame.buffer_mut());

    let [tabs_area, list_area] = area.split(&Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
    ]));

    cx.state.tabs.render(tabs_area, frame.buffer_mut(), &cx);

    let list = {
        let items = if let Some(ref sub) = cx.state.user_subscription {
            sub.feeds
                .iter()
                .map(|feed| feed.url.as_str())
                .map(|url| {
                    Line::from(Span::styled(
                        format!("Url: {url}"),
                        Style::default().fg(Color::Green),
                    ))
                })
                .map(ListItem::new)
                .collect()
        } else {
            vec![]
        };

        List::new(items)
    };

    frame.render_widget(list, list_area);
}

/// Create centered Rect
#[allow(dead_code)]
fn centered(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // get vertically centered rect
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // then centered horizontally
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(layout[1])[1]
}
