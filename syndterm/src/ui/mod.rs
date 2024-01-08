use ratatui::{
    prelude::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, Widget},
    Frame,
};

use crate::{
    application::{Screen, State},
    ui::{tabs::Tab, theme::Theme},
};

pub mod login;
pub mod prompt;
pub mod subscription;
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

    let [tabs_area, content_area, prompt_area] = area.split(&Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ]));

    cx.state.tabs.render(tabs_area, frame.buffer_mut(), &cx);

    background::RgbSwatch.render(content_area, frame.buffer_mut());

    match cx.state.tabs.current() {
        Tab::Subscription => cx
            .state
            .subscription
            .render(content_area, frame.buffer_mut(), &cx),
        Tab::Feeds => {}
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

// steal from https://github.com/ratatui-org/ratatui/blob/main/examples/demo2/colors.rs
mod background {
    use palette::{IntoColor, Okhsv, Srgb};
    use ratatui::{prelude::*, widgets::*};

    /// A widget that renders a color swatch of RGB colors.
    ///
    /// The widget is rendered as a rectangle with the hue changing along the x-axis from 0.0 to 360.0
    /// and the value changing along the y-axis (from 1.0 to 0.0). Each pixel is rendered as a block
    /// character with the top half slightly lighter than the bottom half.
    pub struct RgbSwatch;

    impl Widget for RgbSwatch {
        fn render(self, area: Rect, buf: &mut Buffer) {
            for (yi, y) in (area.top()..area.bottom()).enumerate() {
                let value = area.height as f32 - yi as f32;
                let value_fg = value / (area.height as f32);
                let value_bg = (value - 0.5) / (area.height as f32);
                for (xi, x) in (area.left()..area.right()).enumerate() {
                    let hue = xi as f32 * 360.0 / area.width as f32;
                    let fg = color_from_oklab(hue, Okhsv::max_saturation(), value_fg);
                    let bg = color_from_oklab(hue, Okhsv::max_saturation(), value_bg);
                    buf.get_mut(x, y).set_char('▀').set_fg(fg).set_bg(bg);
                }
            }
        }
    }

    /// Convert a hue and value into an RGB color via the OkLab color space.
    ///
    /// See <https://bottosson.github.io/posts/oklab/> for more details.
    pub fn color_from_oklab(hue: f32, saturation: f32, value: f32) -> Color {
        let color: Srgb = Okhsv::new(hue, saturation, value).into_color();
        let color = color.into_format();
        Color::Rgb(color.red, color.green, color.blue)
    }
}
