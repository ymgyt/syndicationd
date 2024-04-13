use ratatui::style::{
    palette::tailwind::{self, Palette},
    Color, Modifier, Style,
};

#[derive(Clone)]
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
    pub default_icon_fg: Color,
    pub requiment_fg: Color,
}

#[derive(Clone)]
pub struct LoginTheme {
    pub title: Style,
    pub selected_auth_provider_item: Style,
}

#[derive(Clone)]
pub struct ErrorTheme {
    pub message: Style,
}

#[derive(Clone)]
pub struct PromptTheme {
    pub key: Style,
    pub key_desc: Style,
    pub background: Style,
}

#[derive(Clone)]
pub struct SubscriptionTheme {
    pub background: Style,
    pub header: Style,
    pub selected_feed: Style,
}

#[derive(Clone)]
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
                message: Style::new().fg(err).bg(bg),
            },
            default_icon_fg: fg,
            requiment_fg: bg,
        }
    }
    pub fn new() -> Self {
        Self::with_palette(&tailwind::SLATE)
    }
}
