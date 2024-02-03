use ratatui::style::{
    palette::tailwind::{self, Palette},
    Color, Modifier, Style,
};

pub struct Theme {
    pub background: Style,
    pub application_title: Style,
    pub login: LoginTheme,
    pub tabs: Style,
    pub tabs_selected: Style,
    pub prompt: PromptTheme,
    pub subscription: SubscriptionTheme,
    pub entries: EntriesTheme,
    pub error: ErrorTheme,
}

pub struct LoginTheme {
    pub title: Style,
    pub selected_auth_provider_item: Style,
}

pub struct ErrorTheme {
    pub message: Style,
}

pub struct PromptTheme {
    pub key: Style,
    pub key_desc: Style,
    pub background: Style,
}

pub struct SubscriptionTheme {
    pub background: Style,
    pub header: Style,
    pub selected_feed: Style,
}

pub struct EntriesTheme {
    pub background: Style,
    pub header: Style,
    pub selected_entry: Style,
}

impl Theme {
    pub fn with_palette(p: &Palette) -> Self {
        let gray = tailwind::ZINC;

        let bg = p.c950;
        let fg = p.c100;
        let fg_dark = gray.c400;
        let err = tailwind::RED.c600;

        Self {
            background: Style::new().bg(bg),
            application_title: Style::new().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
            login: LoginTheme {
                title: Style::new().add_modifier(Modifier::BOLD),
                selected_auth_provider_item: Style::new().add_modifier(Modifier::BOLD),
            },
            tabs: Style::new().fg(gray.c600).bg(bg),
            tabs_selected: Style::new().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
            prompt: PromptTheme {
                key: Style::new().fg(fg_dark).bg(bg),
                key_desc: Style::new().fg(bg).bg(fg_dark),
                background: Style::new().bg(bg),
            },
            subscription: SubscriptionTheme {
                background: Style::new().bg(bg),
                header: Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                selected_feed: Style::new().add_modifier(Modifier::BOLD),
            },
            entries: EntriesTheme {
                background: Style::new().bg(bg),
                header: Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                selected_entry: Style::new().add_modifier(Modifier::BOLD),
            },
            error: ErrorTheme {
                message: Style::new().fg(err).add_modifier(Modifier::BOLD),
            },
        }
    }
    pub fn new() -> Self {
        Self {
            background: Style::new().bg(DARK_BLUE),
            application_title: Style::new()
                .fg(WHITE)
                .bg(DARK_BLUE)
                .add_modifier(Modifier::BOLD),
            login: LoginTheme {
                title: Style::new().add_modifier(Modifier::BOLD),
                selected_auth_provider_item: Style::new().add_modifier(Modifier::BOLD),
            },
            tabs: Style::new().fg(MID_GRAY).bg(DARK_BLUE),
            tabs_selected: Style::new()
                .fg(WHITE)
                .bg(DARK_BLUE)
                .add_modifier(Modifier::BOLD),
            prompt: PromptTheme {
                key: Style::new().fg(DDARK_BLUE).bg(DARK_GRAY),
                key_desc: Style::new().fg(DARK_GRAY).bg(DDARK_BLUE),
                background: Style::new().bg(DDARK_BLUE),
            },
            subscription: SubscriptionTheme {
                background: Style::new().bg(DARK_BLUE),
                header: Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                selected_feed: Style::new().add_modifier(Modifier::BOLD),
            },
            entries: EntriesTheme {
                background: Style::new().bg(DARK_BLUE),
                header: Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                selected_entry: Style::new().add_modifier(Modifier::BOLD),
            },
            error: ErrorTheme {
                message: Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            },
        }
    }
}

const DARK_BLUE: Color = Color::Rgb(16, 24, 48);
const DDARK_BLUE: Color = Color::Rgb(8, 16, 40);
const DARK_GRAY: Color = Color::Indexed(238);
const MID_GRAY: Color = Color::Indexed(244);
const WHITE: Color = Color::Indexed(255);
