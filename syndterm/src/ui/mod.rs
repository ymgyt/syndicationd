use ratatui::{
    prelude::{Constraint, Layout},
    widgets::{Block, Widget},
    Frame,
};

use crate::{
    application::{Screen, State},
    ui::{
        components::{login, tabs::Tab},
        theme::Theme,
    },
};

pub mod components;
pub mod extension;
pub mod theme;

pub const UNKNOWN_SYMBOL: &str = "???";
pub const TABLE_HIGHLIGHT_SYMBOL: &str = ">> ";

pub struct Context<'a> {
    pub theme: &'a Theme,
}

pub fn render(frame: &mut Frame, mut cx: Context<'_>) {
    let area = frame.size();

    // Background
    Block::new()
        .style(cx.theme.background)
        .render(area, frame.buffer_mut());

    match cx.state.screen {
        Screen::Login => return login::render(frame.size(), frame, &mut cx),
        Screen::Login => return cx.state.login.render(area, frame, &mut cx),
        Screen::Browse => {}
    }

    let [tabs_area, content_area, prompt_area] = area.split(&Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ]));

    cx.state
        .components
        .tabs
        .render(tabs_area, frame.buffer_mut(), &cx);

    match cx.state.components.tabs.current() {
        Tab::Subscription => {
            cx.state
                .components
                .subscription
                .render(content_area, frame.buffer_mut(), &cx)
        }
        Tab::Feeds => cx
            .state
            .components
            .entries
            .render(content_area, frame.buffer_mut(), &cx),
    };

    cx.state
        .components
        .prompt
        .render(prompt_area, frame.buffer_mut(), &cx);
}
