use synd_feed::types::Requirement;

use crate::{
    types,
    ui::components::filter::{
        category::CategoriesState, composed::Composable, CategoryFilterer, ComposedFilterer,
        FilterResult, Filterable, MatcherFilterer,
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
pub(crate) struct RequirementFilterer {
    requirement: Requirement,
}

impl Default for RequirementFilterer {
    fn default() -> Self {
        Self::new(FeedHandler::INITIAL_REQUIREMENT)
    }
}

impl Composable for RequirementFilterer {}

impl RequirementFilterer {
    pub(super) fn new(requirement: Requirement) -> Self {
        Self { requirement }
    }
}

impl Filterable<types::Entry> for RequirementFilterer {
    fn filter(&self, entry: &types::Entry) -> FilterResult {
        if entry.requirement().is_satisfied(self.requirement) {
            FilterResult::Use
        } else {
            FilterResult::Discard
        }
    }
}

impl Filterable<types::Feed> for RequirementFilterer {
    fn filter(&self, feed: &types::Feed) -> FilterResult {
        if feed.requirement().is_satisfied(self.requirement) {
            FilterResult::Use
        } else {
            FilterResult::Discard
        }
    }
}

pub(crate) type FeedFilterer =
    ComposedFilterer<ComposedFilterer<RequirementFilterer, CategoryFilterer>, MatcherFilterer>;
