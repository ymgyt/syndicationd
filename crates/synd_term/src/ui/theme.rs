use ratatui::style::{Color, Modifier, Style, Stylize};

#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,
    pub base: Style,
    pub application_title: Style,
    pub login: LoginTheme,
    pub tabs: Style,
    pub tabs_selected: Style,
    pub prompt: PromptTheme,
    pub subscription: SubscriptionTheme,
    pub entries: EntriesTheme,
    pub error: ErrorTheme,
    pub default_icon_fg: Color,
    pub requirement: RequirementLabelTheme,
    pub selection_popup: SelectionPopup,
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
    pub header: Style,
    pub selected_entry: Style,
    pub summary: Style,
}

#[derive(Clone)]
pub struct RequirementLabelTheme {
    pub must: Color,
    pub should: Color,
    pub may: Color,
    pub fg: Color,
}

#[derive(Clone)]
pub struct SelectionPopup {
    pub highlight: Style,
}

pub struct Palette {
    name: &'static str,
    bg: Color,
    fg: Color,
    fg_inactive: Color,
    fg_focus: Color,
    error: Color,
}

impl Palette {
    pub fn helix() -> Self {
        Self {
            name: "helix",
            bg: Color::Rgb(0x3b, 0x22, 0x4c),
            fg: Color::Rgb(0xa4, 0xa0, 0xe8),
            fg_inactive: Color::Rgb(0x69, 0x7c, 0x81),
            fg_focus: Color::Rgb(0xff, 0xff, 0xff),
            error: Color::Rgb(0xf4, 0x78, 0x68),
        }
    }

    pub fn ferra() -> Self {
        Self {
            name: "ferra",
            bg: Color::Rgb(0x2b, 0x29, 0x2d),
            fg: Color::Rgb(0xfe, 0xcd, 0xb2),
            fg_inactive: Color::Rgb(0x6F, 0x5D, 0x63),
            fg_focus: Color::Rgb(0xff, 0xa0, 0x7a),
            error: Color::Rgb(0xe0, 0x6b, 0x75),
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            name: "solarized_dark",
            bg: Color::Rgb(0x00, 0x2b, 0x36),
            fg: Color::Rgb(0x93, 0xa1, 0xa1),
            fg_inactive: Color::Rgb(0x58, 0x6e, 0x75),
            fg_focus: Color::Rgb(0x26, 0x8b, 0xd2),
            error: Color::Rgb(0xdc, 0x32, 0x2f),
        }
    }
}

impl Theme {
    pub fn with_palette(p: &Palette) -> Self {
        let Palette {
            name,
            bg,
            fg,
            fg_inactive,
            fg_focus,
            error,
        } = *p;

        Self {
            name,
            base: Style::new().bg(bg).fg(fg),
            application_title: Style::new().fg(fg).bg(bg).add_modifier(Modifier::BOLD),
            login: LoginTheme {
                title: Style::new().add_modifier(Modifier::BOLD),
                selected_auth_provider_item: Style::new().add_modifier(Modifier::BOLD),
            },
            tabs: Style::new().fg(fg),
            tabs_selected: Style::new().fg(fg_focus).bold(),
            prompt: PromptTheme {
                key: Style::new().fg(fg_inactive).bg(bg),
                key_desc: Style::new().fg(fg_inactive).bg(bg),
                background: Style::new().bg(bg),
            },
            subscription: SubscriptionTheme {
                background: Style::new().bg(bg),
                header: Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                selected_feed: Style::new().fg(fg_focus).add_modifier(Modifier::BOLD),
            },
            entries: EntriesTheme {
                header: Style::new().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                selected_entry: Style::new().fg(fg_focus).add_modifier(Modifier::BOLD),
                summary: Style::new().fg(fg),
            },
            error: ErrorTheme {
                message: Style::new().fg(error).bg(bg),
            },
            default_icon_fg: fg,
            requirement: RequirementLabelTheme {
                must: bg,
                should: bg,
                may: bg,
                fg,
            },
            selection_popup: SelectionPopup {
                highlight: Style::new().bg(Color::Yellow).fg(bg),
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::with_palette(&Palette::ferra())
    }
}
