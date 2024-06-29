use std::collections::HashMap;

use synd_feed::types::{Category, Requirement};

use crate::{
    matcher::Matcher,
    types,
    ui::components::filter::{
        category::{CategoriesState, FilterCategoryState},
        FilterResult, Filterable,
    },
};

#[derive(Debug)]
pub(crate) struct FeedHandler {
    pub(super) requirement: Requirement,
    pub(super) categories_state: CategoriesState,
}

impl FeedHandler {
    const INITIAL_REQUIREMENT: Requirement = Requirement::May;
    pub(super) fn new() -> Self {
        Self {
            requirement: Self::INITIAL_REQUIREMENT,
            categories_state: CategoriesState::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct FeedFilterer {
    pub(super) requirement: Requirement,
    pub(super) categories: HashMap<Category<'static>, FilterCategoryState>,
    pub(super) matcher: Matcher,
}

impl Default for FeedFilterer {
    fn default() -> Self {
        Self {
            requirement: FeedHandler::INITIAL_REQUIREMENT,
            categories: HashMap::new(),
            matcher: Matcher::new(),
        }
    }
}

impl Filterable<types::Entry> for FeedFilterer {
    fn filter(&self, entry: &types::Entry) -> FilterResult {
        self.filter_entry(entry)
    }
}

impl Filterable<types::Feed> for FeedFilterer {
    fn filter(&self, feed: &types::Feed) -> FilterResult {
        self.filter_feed(feed)
    }
}

impl FeedFilterer {
    pub fn filter_entry(&self, entry: &types::Entry) -> FilterResult {
        if !entry.requirement().is_satisfied(self.requirement) {
            return FilterResult::Discard;
        }
        if let Some(FilterCategoryState::Inactive) = self.categories.get(entry.category()) {
            return FilterResult::Discard;
        }
        if self
            .matcher
            .r#match(entry.title.as_deref().unwrap_or_default())
            || self
                .matcher
                .r#match(entry.feed_title.as_deref().unwrap_or_default())
        {
            FilterResult::Use
        } else {
            FilterResult::Discard
        }
    }

    pub fn filter_feed(&self, feed: &types::Feed) -> FilterResult {
        if !feed.requirement().is_satisfied(self.requirement) {
            return FilterResult::Discard;
        }
        if let Some(FilterCategoryState::Inactive) = self.categories.get(feed.category()) {
            return FilterResult::Discard;
        }
        if self
            .matcher
            .r#match(feed.title.as_deref().unwrap_or_default())
            || self
                .matcher
                .r#match(feed.website_url.as_deref().unwrap_or_default())
        {
            return FilterResult::Use;
        }
        FilterResult::Discard
    }
}
