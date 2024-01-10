use ratatui::style::{Color, Modifier, Style};

pub struct Theme {
    pub background: Style,
    pub application_title: Style,
    pub tabs: Style,
    pub tabs_selected: Style,
    pub prompt: Prompt,
    pub error: Error,
}

pub struct Error {
    pub message: Style,
}

pub struct Prompt {
    pub key: Style,
    pub key_desc: Style,
    pub background: Style,
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
            prompt: Prompt {
                key: Style::new().fg(DDARK_BLUE).bg(DARK_GRAY),
                key_desc: Style::new().fg(DARK_GRAY).bg(DDARK_BLUE),
                background: Style::new().bg(DDARK_BLUE),
            },
            error: Error {
                message: Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            },
        }
    }
}

const DARK_BLUE: Color = Color::Rgb(16, 24, 48);
const DDARK_BLUE: Color = Color::Rgb(8, 16, 40);
// const BLACK: Color = Color::Indexed(232);
const DARK_GRAY: Color = Color::Indexed(238);
const MID_GRAY: Color = Color::Indexed(244);
const WHITE: Color = Color::Indexed(255);
