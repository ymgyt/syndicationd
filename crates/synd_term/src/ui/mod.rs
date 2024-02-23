use crate::{application::InFlight, ui::theme::Theme};

pub mod components;
pub mod extension;
pub mod theme;
pub mod widgets;

pub const UNKNOWN_SYMBOL: &str = "-";
pub const TABLE_HIGHLIGHT_SYMBOL: &str = ">> ";

pub struct Context<'a> {
    pub theme: &'a Theme,
    pub in_flight: &'a InFlight,
}
