use crate::ui::theme::Theme;

pub mod components;
pub mod extension;
pub mod theme;

pub const UNKNOWN_SYMBOL: &str = "???";
pub const TABLE_HIGHLIGHT_SYMBOL: &str = ">> ";

pub struct Context<'a> {
    pub theme: &'a Theme,
}
