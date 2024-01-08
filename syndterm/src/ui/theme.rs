use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub background: Style,
    pub application_title: Style,
    pub tabs: Style,
    pub tabs_selected: Style,
}

impl Theme {
    pub fn new() -> Self {
        Self {
            background: Style::new().bg(DARK_BLUE),
            application_title: Style::new()
                .fg(WHITE)
                .bg(DARK_BLUE)
                .add_modifier(Modifier::BOLD),
            tabs: Style::new().fg(MID_GRAY).bg(DARK_BLUE),
            tabs_selected: Style::new()
                .fg(WHITE)
                .bg(DARK_BLUE)
                .add_modifier(Modifier::BOLD),
        }
    }
}

const DARK_BLUE: Color = Color::Rgb(16, 24, 48);
const MID_GRAY: Color = Color::Indexed(244);
const WHITE: Color = Color::Indexed(255);
