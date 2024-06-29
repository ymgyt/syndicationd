use std::collections::HashMap;
use synd_feed::types::Category;

use crate::{
    types::github::Notification,
    ui::components::filter::{
        category::{CategoriesState, FilterCategoryState},
        FilterResult, Filterable,
    },
};

#[derive(Debug)]
pub(super) struct GhNotificationHandler {
    pub(super) categories_state: CategoriesState,
}

impl GhNotificationHandler {
    pub(super) fn new() -> Self {
        Self {
            categories_state: CategoriesState::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct GhNotificationFilterer {
    pub(super) categories: HashMap<Category<'static>, FilterCategoryState>,
}

impl Filterable<Notification> for GhNotificationFilterer {
    fn filter(&self, n: &Notification) -> FilterResult {
        if !self.categories.is_empty()
            && n.categories()
                .filter_map(|c| self.categories.get(c))
                .all(|state| !state.is_active())
        {
            return FilterResult::Discard;
        }
        FilterResult::Use
    }
}
