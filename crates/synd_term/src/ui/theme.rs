use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use strum::Display;
pub use tuiframework::theme::Palette;
use tuiframework::theme::Styles;

/// The background used for the main surface of the TUI.
#[derive(Clone, Copy, Debug, Default, Deserialize, Display, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum ThemeBackground {
    #[default]
    Theme,
    Transparent,
}

#[derive(Clone)]
pub struct Theme {
    pub name: &'static str,
    pub(crate) background: ThemeBackground,
    dark_label_fg: Color,
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

impl Theme {
    pub fn with_palette(palette: Palette) -> Self {
        Self::new(palette, ThemeBackground::Theme)
    }

    pub fn new(palette: Palette, background: ThemeBackground) -> Self {
        let bg = match background {
            ThemeBackground::Theme => palette.background,
            ThemeBackground::Transparent => Color::Reset,
        };
        let common = Styles::from(Palette {
            background: bg,
            ..palette
        });

        Self {
            name: palette.name,
            background,
            dark_label_fg: palette.background,
            base: common.base,
            application_title: common.title,
            login: LoginTheme {
                title: Style::new().add_modifier(Modifier::BOLD),
                selected_auth_provider_item: Style::new().add_modifier(Modifier::BOLD),
            },
            tabs: Style::new().fg(palette.foreground),
            tabs_selected: common.selected,
            prompt: PromptTheme {
                key: common.muted,
                key_desc: common.muted,
                background: Style::new().bg(bg),
            },
            subscription: SubscriptionTheme {
                background: Style::new().bg(bg),
                header: common.header,
                selected_feed: common.selected,
            },
            entries: EntriesTheme {
                header: common.header,
                selected_entry: common.selected,
                summary: Style::new().fg(palette.foreground),
            },
            error: ErrorTheme {
                message: common.error,
            },
            default_icon_fg: palette.foreground,
            requirement: RequirementLabelTheme {
                must: bg,
                should: bg,
                may: bg,
                fg: palette.foreground,
            },
            selection_popup: SelectionPopup {
                highlight: Style::new().bg(Color::Yellow).fg(palette.background),
            },
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Theme::with_palette(Palette::ferra())
    }
}

impl Theme {
    pub(crate) fn contrast_fg_from_luminance(&self, luminance: f64) -> Color {
        if luminance > 0.5 {
            self.dark_label_fg
        } else {
            self.base.fg.unwrap_or_default()
        }
    }
}
