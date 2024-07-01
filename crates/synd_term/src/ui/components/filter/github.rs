use std::collections::HashMap;
use synd_feed::types::Category;

use crate::{
    types::github::Notification,
    ui::components::{
        filter::{
            category::{CategoriesState, FilterCategoryState},
            FilterResult, Filterable,
        },
        gh_notifications::GhNotificationFilterOptions,
    },
};

#[derive(Debug)]
pub(super) struct GhNotificationHandler {
    pub(super) categories_state: CategoriesState,
    pub(super) filter_options: GhNotificationFilterOptions,
}

impl GhNotificationHandler {
    pub(super) fn new() -> Self {
        Self {
            categories_state: CategoriesState::new(),
            filter_options: GhNotificationFilterOptions::default(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct GhNotificationFilterer {
    pub(super) categories: HashMap<Category<'static>, FilterCategoryState>,
    pub(super) options: GhNotificationFilterOptions,
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

        // unread and participating are handled in rest api
        if let Some(visibility) = self.options.visibility {
            if visibility != n.repository.visibility {
                return FilterResult::Discard;
            }
        }

        FilterResult::Use
    }
}
