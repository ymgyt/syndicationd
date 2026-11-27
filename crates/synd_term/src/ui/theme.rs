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
    palette_background: Color,
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
        Self::new(palette)
    }

    /// Creates a theme with the palette's own background.
    pub fn new(palette: Palette) -> Self {
        let bg = palette.background;
        let common = Styles::from(palette);

        Self {
            name: palette.name,
            background: ThemeBackground::Theme,
            palette_background: bg,
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
                key_desc: common
                    .base
                    .fg(blend_rgb(palette.foreground, palette.muted, 40)
                        .unwrap_or(palette.foreground)),
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

    /// Selects the background used for the main surface.
    #[must_use]
    pub fn with_background(mut self, background: ThemeBackground) -> Self {
        let color = match background {
            ThemeBackground::Theme => self.palette_background,
            ThemeBackground::Transparent => Color::Reset,
        };
        self.background = background;
        for style in [
            &mut self.base,
            &mut self.application_title,
            &mut self.prompt.key,
            &mut self.prompt.key_desc,
            &mut self.prompt.background,
            &mut self.subscription.background,
            &mut self.error.message,
        ] {
            style.bg = Some(color);
        }
        self.requirement.must = color;
        self.requirement.should = color;
        self.requirement.may = color;
        self
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::new(Palette::ferra())
    }
}

impl Theme {
    pub(crate) fn backdrop_style(&self, foreground: Color, background: Color) -> Style {
        let base = match background {
            Color::Rgb(..) => background,
            _ => self.palette_background,
        };
        let dark = blend_rgb(base, Color::Rgb(0, 0, 0), 62).unwrap_or(Color::Black);
        let style = Style::new().bg(dark);
        // Keep the hues of icons and selections while lowering their contrast.
        let muted = blend_rgb(foreground, base, 46).and_then(|fg| blend_rgb(fg, dark, 32));
        match muted {
            Some(fg) => style.fg(fg).remove_modifier(Modifier::DIM),
            None => style.add_modifier(Modifier::DIM),
        }
    }

    pub(crate) fn input_surface(&self) -> Style {
        let underline = self.prompt.key.fg.unwrap_or(Color::Reset);
        let style = Style::new()
            .add_modifier(Modifier::UNDERLINED)
            .underline_color(underline);
        match (self.base.bg, self.prompt.key.fg) {
            (Some(Color::Rgb(r, g, b)), Some(Color::Rgb(mr, mg, mb))) => {
                let tint = |base: u8, muted| base.midpoint(base.midpoint(muted));
                style.bg(Color::Rgb(tint(r, mr), tint(g, mg), tint(b, mb)))
            }
            _ => style,
        }
    }

    pub(crate) fn input_placeholder(&self) -> Style {
        let foreground = self
            .base
            .fg
            .zip(self.prompt.key.fg)
            .and_then(|(fg, muted)| blend_rgb(fg, muted, 50));
        match foreground {
            Some(fg) => Style::new().fg(fg),
            None => Style::new().add_modifier(Modifier::DIM),
        }
    }

    pub(crate) fn contrast_fg_from_luminance(&self, luminance: f64) -> Color {
        if luminance > 0.5 {
            self.palette_background
        } else {
            self.base.fg.unwrap_or_default()
        }
    }
}

fn blend_rgb(from: Color, to: Color, percent: u16) -> Option<Color> {
    let (Color::Rgb(r, g, b), Color::Rgb(tr, tg, tb)) = (from, to) else {
        return None;
    };
    let mix = |from: u8, to: u8| {
        let sum = u16::from(from) * (100 - percent) + u16::from(to) * percent;
        u8::try_from((sum + 50) / 100).expect("weighted average fits in one channel")
    };
    Some(Color::Rgb(mix(r, tr), mix(g, tg), mix(b, tb)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_background_can_be_restored_after_applying_transparency() {
        let palette = Palette::dracula();
        let theme = Theme::new(palette);
        assert_eq!(theme.base.bg, Some(palette.background));

        let theme = theme.with_background(ThemeBackground::Transparent);
        assert_eq!(theme.prompt.background.bg, Some(Color::Reset));
        assert_eq!(theme.requirement.must, Color::Reset);
        assert_eq!(theme.contrast_fg_from_luminance(1.0), palette.background);

        let theme = theme.with_background(ThemeBackground::Theme);
        assert_eq!(theme.base.bg, Some(palette.background));
        assert_eq!(theme.prompt.background.bg, Some(palette.background));
        assert_eq!(theme.requirement.must, palette.background);
    }
}
