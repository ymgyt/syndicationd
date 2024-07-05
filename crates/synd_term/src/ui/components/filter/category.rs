use std::collections::{HashMap, HashSet};

use synd_feed::types::Category;

use crate::{
    application::Populate,
    config::{Categories, Icon},
    types::{self, github::Notification},
    ui::{
        self,
        components::filter::{Composable, FilterResult, Filterable},
    },
};

#[allow(dead_code)]
static LABELS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FilterCategoryState {
    Active,
    Inactive,
}

impl FilterCategoryState {
    pub(super) fn toggle(self) -> Self {
        match self {
            FilterCategoryState::Active => FilterCategoryState::Inactive,
            FilterCategoryState::Inactive => FilterCategoryState::Active,
        }
    }
    pub(super) fn is_active(self) -> bool {
        self == FilterCategoryState::Active
    }
}

#[derive(Debug)]
pub(super) struct CategoryState {
    pub(super) label: char,
    pub(super) state: FilterCategoryState,
    pub(super) icon: Icon,
}

#[derive(Debug)]
pub(super) struct CategoriesState {
    // TODO: make private
    pub(super) categories: Vec<Category<'static>>,
    pub(super) state: HashMap<Category<'static>, CategoryState>,
}

impl CategoriesState {
    pub(super) fn new() -> Self {
        Self {
            categories: Vec::new(),
            state: HashMap::new(),
        }
    }

    pub(super) fn update(
        &mut self,
        config: &Categories,
        populate: Populate,
        categories: impl IntoIterator<Item = Category<'static>>,
    ) {
        let new = categories.into_iter().collect::<HashSet<_>>();
        let mut prev = self.categories.drain(..).collect::<HashSet<_>>();

        let mut new_categories = match populate {
            Populate::Replace => {
                let should_remove = prev.difference(&new);
                let should_create = new.difference(&prev);

                for c in should_remove {
                    self.state.remove(c);
                }
                for c in should_create {
                    self.state.insert(
                        c.clone(),
                        CategoryState {
                            label: ' ',
                            icon: config.icon(c).unwrap_or_else(|| ui::default_icon()).clone(),
                            state: FilterCategoryState::Active,
                        },
                    );
                }

                new.into_iter().collect::<Vec<_>>()
            }
            Populate::Append => {
                let should_create = new.difference(&prev);
                for c in should_create {
                    self.state.insert(
                        c.clone(),
                        CategoryState {
                            label: ' ',
                            icon: config.icon(c).unwrap_or_else(|| ui::default_icon()).clone(),
                            state: FilterCategoryState::Active,
                        },
                    );
                }

                prev.extend(new);
                prev.into_iter().collect::<Vec<_>>()
            }
        };

        new_categories.sort_unstable();
        self.categories = new_categories;
        self.assigine_category_labels();
    }

    fn assigine_category_labels(&mut self) {
        self.categories
            .iter()
            .zip(LABELS)
            .for_each(|(category, label)| {
                self.state.get_mut(category).unwrap().label = *label;
            });
    }

    pub(super) fn clear(&mut self) {
        self.categories.clear();
        self.state.clear();
    }
}

#[derive(Default, Clone, Debug)]
pub(crate) struct CategoryFilterer {
    state: HashMap<Category<'static>, FilterCategoryState>,
}

impl Composable for CategoryFilterer {}

impl CategoryFilterer {
    pub(crate) fn new(state: HashMap<Category<'static>, FilterCategoryState>) -> Self {
        Self { state }
    }

    fn filter_by_category(&self, category: &Category<'_>) -> FilterResult {
        match self.state.get(category) {
            Some(FilterCategoryState::Inactive) => FilterResult::Discard,
            _ => FilterResult::Use,
        }
    }
}

impl Filterable<types::Entry> for CategoryFilterer {
    fn filter(&self, entry: &types::Entry) -> super::FilterResult {
        self.filter_by_category(entry.category())
    }
}

impl Filterable<types::Feed> for CategoryFilterer {
    fn filter(&self, feed: &types::Feed) -> super::FilterResult {
        self.filter_by_category(feed.category())
    }
}

impl Filterable<Notification> for CategoryFilterer {
    fn filter(&self, n: &Notification) -> super::FilterResult {
        if !self.state.is_empty()
            && n.categories()
                .filter_map(|c| self.state.get(c))
                .all(|state| !state.is_active())
        {
            FilterResult::Discard
        } else {
            FilterResult::Use
        }
    }
}
