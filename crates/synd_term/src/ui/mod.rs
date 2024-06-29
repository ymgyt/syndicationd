use std::{str::FromStr, sync::OnceLock};

use ratatui::style::{Color, Modifier};
use synd_feed::types::{Category, Requirement};

use crate::{
    application::{InFlight, TerminalFocus},
    config::{Categories, Icon, IconColor},
    types::Time,
    ui::{components::tabs::Tab, theme::Theme},
};

pub mod components;
pub mod extension;
pub mod theme;
pub mod widgets;

mod icon;
pub(crate) use icon::icon;

pub const UNKNOWN_SYMBOL: &str = "-";
pub const TABLE_HIGHLIGHT_SYMBOL: &str = " ";
pub const DEFAULT_REQUIREMNET: Requirement = Requirement::Should;

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
    pub in_flight: &'a InFlight,
    pub categories: &'a Categories,
    pub(crate) now: Time,
    pub(crate) focus: TerminalFocus,
    pub(crate) tab: Tab,
}

impl<'a> Context<'a> {
    fn table_highlight_modifier(&self) -> Modifier {
        match self.focus {
            TerminalFocus::Gained => Modifier::empty(),
            TerminalFocus::Lost => Modifier::DIM,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_icon_is_not_empty() {
        assert!(!default_icon().symbol().is_empty());
    }
}
