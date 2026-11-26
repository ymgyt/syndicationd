use std::{str::FromStr, sync::OnceLock};
use tuiframework::TerminalFocus;

use ratatui::style::{Color, Modifier, Style};
use synd_feed::types::{Category, Requirement};

use crate::{
    config::{Categories, Icon, IconColor},
    types::Time,
    ui::theme::Theme,
};

pub mod extension;
pub mod theme;
pub mod widgets;

mod icon;
pub(crate) use icon::icon;

pub const UNKNOWN_SYMBOL: &str = "-";
pub const TABLE_HIGHLIGHT_SYMBOL: &str = " ";
pub const DEFAULT_REQUIREMENT: Requirement = Requirement::Should;

pub fn default_category() -> &'static Category<'static> {
    static DEFAULT_CATEGORY: OnceLock<Category<'static>> = OnceLock::new();

    DEFAULT_CATEGORY.get_or_init(|| Category::new("default").unwrap())
}

pub fn default_icon() -> &'static Icon {
    static DEFAULT_ICON: OnceLock<Icon> = OnceLock::new();

    DEFAULT_ICON.get_or_init(|| {
        Icon::new("󰎞").with_color(IconColor::new(Color::from_str("dark gray").unwrap()))
    })
}

pub struct Context<'a> {
    pub theme: &'a Theme,
    pub categories: &'a Categories,
    pub(crate) now: Time,
    pub(crate) focus: TerminalFocus,
}

impl Context<'_> {
    fn table_selection_style(&self) -> Style {
        let selected = self.theme.entries.selected_entry;
        match self.focus {
            TerminalFocus::Gained => selected,
            TerminalFocus::Lost => selected.add_modifier(Modifier::DIM),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_icon_is_not_empty() {
        assert_ne!(default_icon().symbol(), "");
    }
}
