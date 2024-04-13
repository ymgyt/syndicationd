use std::sync::OnceLock;

use synd_feed::types::{Category, Requirement};

use crate::{application::InFlight, config::Categories, ui::theme::Theme};

pub mod components;
pub mod extension;
pub mod theme;
pub mod widgets;

pub const UNKNOWN_SYMBOL: &str = "-";
pub const TABLE_HIGHLIGHT_SYMBOL: &str = " ";
pub const DEFAULT_REQUIREMNET: Requirement = Requirement::Should;

pub fn default_category() -> &'static Category<'static> {
    static DEFAULT_CATEGORY: OnceLock<Category<'static>> = OnceLock::new();

    DEFAULT_CATEGORY.get_or_init(|| Category::new("default").unwrap())
}

pub struct Context<'a> {
    pub theme: &'a Theme,
    pub in_flight: &'a InFlight,
    pub categories: &'a Categories,
}
