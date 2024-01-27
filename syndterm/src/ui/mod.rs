use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Widget},
    Frame,
};

use crate::{
    application::{Screen, State},
    ui::{tabs::Tab, theme::Theme},
};

pub mod entries;
pub mod login;
pub mod prompt;
pub mod subscription;
pub mod tabs;
pub mod theme;

pub const UNKNOWN_SYMBOL: &str = "???";
pub const TABLE_HIGHLIGHT_SYMBOL: &str = ">> ";

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

    let [tabs_area, content_area, prompt_area] = area.split(&Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ]));

    cx.state.tabs.render(tabs_area, frame.buffer_mut(), &cx);

    match cx.state.tabs.current() {
        Tab::Subscription => cx
            .state
            .subscription
            .render(content_area, frame.buffer_mut(), &cx),
        Tab::Feeds => cx
            .state
            .entries
            .render(content_area, frame.buffer_mut(), &cx),
    };

    cx.state.prompt.render(prompt_area, frame.buffer_mut(), &cx);
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
