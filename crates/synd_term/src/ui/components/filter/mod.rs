use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Padding, Widget},
};
use synd_feed::types::{Category, Requirement};

use crate::{
    application::{Direction, Populate},
    client::github::{FetchNotificationInclude, FetchNotificationParticipating},
    command::Command,
    config::Categories,
    keymap::{KeyTrie, Keymap},
    matcher::Matcher,
    types::{
        self, RequirementExt,
        github::{PullRequestState, Reason, RepoVisibility},
    },
    ui::{
        Context,
        components::{
            filter::{
                category::{CategoriesState, FilterCategoryState},
                feed::RequirementFilterer,
                github::GhNotificationHandler,
            },
            gh_notifications::GhNotificationFilterOptions,
            tabs::Tab,
        },
        icon,
        widgets::prompt::{Prompt, RenderCursor},
    },
};

mod feed;
pub(crate) use feed::{FeedFilterer, FeedHandler};

mod github;

mod category;
pub(crate) use category::CategoryFilterer;

mod composed;
pub(crate) use composed::{Composable, ComposedFilterer};

mod matcher;
pub(crate) use matcher::MatcherFilterer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterLane {
    Feed,
    GhNotification,
}

impl From<Tab> for FilterLane {
    fn from(tab: Tab) -> Self {
        match tab {
            Tab::Entries | Tab::Feeds => FilterLane::Feed,
            Tab::GitHub => FilterLane::GhNotification,
        }
    }
}

pub(crate) type CategoryAndMatcherFilterer = ComposedFilterer<CategoryFilterer, MatcherFilterer>;

#[derive(Clone, Debug)]
pub(crate) enum Filterer {
    Feed(FeedFilterer),
    GhNotification(CategoryAndMatcherFilterer),
}

pub(crate) trait Filterable<T> {
    fn filter(&self, item: &T) -> FilterResult;
}

#[derive(Clone, Debug, Copy, PartialEq, Eq)]
pub(crate) enum FilterResult {
    Use,
    Discard,
}

#[derive(Debug)]
pub(crate) struct Filter {
    state: State,
    feed: FeedHandler,
    gh_notification: GhNotificationHandler,

    prompt: Rc<RefCell<Prompt>>,
    matcher: Matcher,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    CategoryFiltering(FilterLane),
    SearchFiltering,
}

impl Filter {
    pub fn new() -> Self {
        Self {
            state: State::Normal,
            prompt: Rc::new(RefCell::new(Prompt::new())),
            feed: FeedHandler::new(),
            gh_notification: GhNotificationHandler::new(),
            matcher: Matcher::new(),
        }
    }

    #[must_use]
    pub fn activate_search_filtering(&mut self) -> Rc<RefCell<Prompt>> {
        self.state = State::SearchFiltering;
        self.prompt.clone()
    }

    pub fn is_search_active(&self) -> bool {
        self.state == State::SearchFiltering
    }

    #[must_use]
    pub fn activate_category_filtering(&mut self, lane: FilterLane) -> Keymap {
        self.state = State::CategoryFiltering(lane);
        let mut map = self.categories_state_from_lane(lane).state.iter().fold(
            HashMap::new(),
            |mut map, (category, state)| {
                let key = KeyEvent::new(KeyCode::Char(state.label), KeyModifiers::NONE);
                let command = Command::ToggleFilterCategory {
                    lane,
                    category: category.clone(),
                };
                map.insert(key, KeyTrie::Command(command));
                map
            },
        );
        map.insert(
            KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE),
            KeyTrie::Command(Command::ActivateAllFilterCategories { lane }),
        );
        map.insert(
            KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE),
            KeyTrie::Command(Command::DeactivateAllFilterCategories { lane }),
        );
        Keymap::from_map(crate::keymap::KeymapId::CategoryFiltering, map)
    }

    fn categories_state_from_lane(&self, lane: FilterLane) -> &CategoriesState {
        match lane {
            FilterLane::Feed => &self.feed.categories_state,
            FilterLane::GhNotification => &self.gh_notification.categories_state,
        }
    }

    fn categories_state_from_lane_mut(&mut self, lane: FilterLane) -> &mut CategoriesState {
        match lane {
            FilterLane::Feed => &mut self.feed.categories_state,
            FilterLane::GhNotification => &mut self.gh_notification.categories_state,
        }
    }

    pub fn deactivate_filtering(&mut self) {
        self.state = State::Normal;
    }

    #[must_use]
    pub fn move_requirement(&mut self, direction: Direction) -> Filterer {
        self.feed.requirement = match direction {
            Direction::Left => {
                if self.feed.requirement == Requirement::Must {
                    Requirement::May
                } else {
                    self.feed.requirement.up()
                }
            }
            Direction::Right => {
                if self.feed.requirement == Requirement::May {
                    Requirement::Must
                } else {
                    self.feed.requirement.down()
                }
            }
            _ => self.feed.requirement,
        };

        Filterer::Feed(self.feed_filterer())
    }

    #[must_use]
    pub fn toggle_category_state(
        &mut self,
        category: &Category<'static>,
        lane: FilterLane,
    ) -> Filterer {
        if let Some(category_state) = self
            .categories_state_from_lane_mut(lane)
            .state
            .get_mut(category)
        {
            category_state.state = category_state.state.toggle();
        }

        self.filterer(lane)
    }

    #[must_use]
    pub fn activate_all_categories_state(&mut self, lane: FilterLane) -> Filterer {
        self.categories_state_from_lane_mut(lane)
            .state
            .iter_mut()
            .for_each(|(_, state)| state.state = FilterCategoryState::Active);

        self.filterer(lane)
    }

    #[must_use]
    pub fn deactivate_all_categories_state(&mut self, lane: FilterLane) -> Filterer {
        self.categories_state_from_lane_mut(lane)
            .state
            .iter_mut()
            .for_each(|(_, state)| state.state = FilterCategoryState::Inactive);

        self.filterer(lane)
    }

    #[must_use]
    pub(crate) fn filterer(&self, lane: FilterLane) -> Filterer {
        match lane {
            FilterLane::Feed => Filterer::Feed(self.feed_filterer()),
            FilterLane::GhNotification => Filterer::GhNotification(self.gh_notification_filterer()),
        }
    }

    #[must_use]
    fn feed_filterer(&self) -> FeedFilterer {
        RequirementFilterer::new(self.feed.requirement)
            .and_then(Self::category_filterer(&self.feed.categories_state))
            .and_then(self.matcher_filterer())
    }

    #[must_use]
    fn gh_notification_filterer(&self) -> CategoryAndMatcherFilterer {
        Self::category_filterer(&self.gh_notification.categories_state)
            .and_then(self.matcher_filterer())
    }

    #[must_use]
    fn category_filterer(categories: &CategoriesState) -> CategoryFilterer {
        CategoryFilterer::new(
            categories
                .state
                .iter()
                .map(|(c, state)| (c.clone(), state.state))
                .collect(),
        )
    }

    #[must_use]
    fn matcher_filterer(&self) -> MatcherFilterer {
        let mut matcher = self.matcher.clone();
        matcher.update_needle(self.prompt.borrow().line());
        MatcherFilterer::new(matcher)
    }

    pub fn update_categories(
        &mut self,
        config: &Categories,
        populate: Populate,
        entries: &[types::Entry],
    ) {
        self.feed.categories_state.update(
            config,
            populate,
            entries.iter().map(types::Entry::category).cloned(),
        );
    }

    pub fn update_gh_notification_categories(
        &mut self,
        config: &Categories,
        populate: Populate,
        categories: impl IntoIterator<Item = Category<'static>>,
    ) {
        self.gh_notification
            .categories_state
            .update(config, populate, categories);
    }

    pub(crate) fn clear_gh_notifications_categories(&mut self) {
        self.gh_notification.categories_state.clear();
    }
}

pub(super) struct FilterContext<'a> {
    pub(super) ui: &'a Context<'a>,
    pub(super) gh_options: &'a GhNotificationFilterOptions,
}

impl Filter {
    pub(super) fn render(&self, area: Rect, buf: &mut Buffer, cx: &FilterContext<'_>) {
        let area = Block::new()
            .padding(Padding {
                left: 2,
                right: 1,
                top: 0,
                bottom: 0,
            })
            .inner(area);
        let vertical = Layout::vertical([Constraint::Length(2), Constraint::Length(1)]);
        let [filter_area, search_area] = vertical.areas(area);

        let lane = cx.ui.tab.into();
        self.render_filter(filter_area, buf, cx, lane);
        self.render_search(search_area, buf, cx.ui, lane);
    }

    #[allow(unstable_name_collisions)]
    fn render_filter(
        &self,
        area: Rect,
        buf: &mut Buffer,
        cx: &FilterContext<'_>,
        lane: FilterLane,
    ) {
        let mut spans = vec![Span::from(concat!(icon!(filter), " Filter")).dim()];

        match lane {
            FilterLane::Feed => {
                let mut r = self.feed.requirement.label(&cx.ui.theme.requirement);
                if r.content == "MAY" {
                    r = r.dim();
                }
                spans.extend([Span::from("    "), r, Span::from("  ")]);
            }
            FilterLane::GhNotification => {
                let options = cx.gh_options;
                let mut unread = Span::from("Unread");
                if options.include == FetchNotificationInclude::All {
                    unread = unread.dim();
                }

                let mut participating = Span::from("Participating");
                if options.participating == FetchNotificationParticipating::All {
                    participating = participating.dim();
                }

                let visibility = match options.visibility {
                    Some(RepoVisibility::Public) => Some(Span::from("Public")),
                    Some(RepoVisibility::Private) => Some(Span::from("Private")),
                    None => None,
                };

                spans.extend([
                    Span::from("  "),
                    unread,
                    Span::from("  "),
                    participating,
                    Span::from("  "),
                ]);
                if let Some(visibility) = visibility {
                    spans.extend([visibility, Span::from("  ")]);
                }

                let pr_conditions = options
                    .pull_request_conditions
                    .iter()
                    .map(|cond| match cond {
                        PullRequestState::Open => Span::from("Open"),
                        PullRequestState::Merged => Span::from("Merged"),
                        PullRequestState::Closed => Span::from("Closed"),
                    })
                    .collect::<Vec<_>>();
                if !pr_conditions.is_empty() {
                    spans.extend(pr_conditions.into_iter().intersperse(Span::from(" ")));
                    spans.push(Span::from("  "));
                }

                let reasons = options
                    .reasons
                    .iter()
                    .filter_map(|reason| match reason {
                        Reason::Mention | Reason::TeamMention => Some(Span::from("Mentioned")),
                        Reason::ReviewRequested => Some(Span::from("ReviewRequested")),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if !reasons.is_empty() {
                    spans.extend(reasons.into_iter().intersperse(Span::from(" ")));
                    spans.push(Span::from("  "));
                }
            }
        }
        let status_line = Line::from(spans);
        #[allow(clippy::cast_possible_truncation)]
        let horizontal = Layout::horizontal([
            Constraint::Length(status_line.width() as u16),
            Constraint::Fill(1),
        ]);
        let [status_area, categories_area] = horizontal.areas(area);

        status_line.render(status_area, buf);

        let (categories, categories_state) = match lane {
            FilterLane::Feed => (
                &self.feed.categories_state.categories,
                &self.feed.categories_state.state,
            ),
            FilterLane::GhNotification => (
                &self.gh_notification.categories_state.categories,
                &self.gh_notification.categories_state.state,
            ),
        };

        let mut spans = vec![];

        let is_active = matches!(self.state, State::CategoryFiltering(active) if active == lane);
        for c in categories {
            let state = categories_state
                .get(c)
                .expect("CategoryState is not found. THIS IS A BUG");
            let mut icon_span = Span::from(state.icon.symbol());
            if let Some(fg) = state.icon.color() {
                icon_span = icon_span.fg(fg);
            }
            if state.state == FilterCategoryState::Inactive {
                icon_span = icon_span.dim();
            }
            spans.push(icon_span);

            if is_active {
                spans.push(Span::from(" "));
                let mut s = Span::from(state.label.to_string());
                if state.state == FilterCategoryState::Active {
                    s = s.underlined();
                } else {
                    s = s.dim();
                }
                spans.push(s);
                spans.push(Span::from(" "));
            } else {
                spans.push(Span::from("   "));
            }
        }
        if is_active {
            spans.push(Span::from("(Esc/+/-)").dim());
        }
        Line::from(spans).render(categories_area, buf);
    }

    fn render_search(&self, area: Rect, buf: &mut Buffer, _cx: &Context<'_>, lane: FilterLane) {
        let mut spans = vec![];
        let mut label = Span::from(concat!(icon!(search), " Search"));
        if self.state != State::SearchFiltering {
            label = label.dim();
        }
        spans.push(label);
        {
            let padding = match lane {
                FilterLane::Feed => "   ",
                FilterLane::GhNotification => " ",
            };
            spans.push(Span::from(padding));
        }

        let search = Line::from(spans);
        let margin = search.width() + 1;
        search.render(area, buf);

        let prompt_area = Rect {
            #[allow(clippy::cast_possible_truncation)]
            x: area.x + margin as u16,
            ..area
        };
        let render_cursor = if self.state == State::SearchFiltering {
            RenderCursor::Enable
        } else {
            RenderCursor::Disable
        };
        self.prompt.borrow().render(prompt_area, buf, render_cursor);
    }
}

#[cfg(test)]
mod tests {
    use fake::{Fake, Faker};

    use crate::types::Feed;

    use super::*;

    #[test]
    fn filter_match_feed_url() {
        let mut matcher = Matcher::new();
        matcher.update_needle("ymgyt");
        let filter = RequirementFilterer::new(Requirement::May)
            .and_then(CategoryFilterer::new(HashMap::new()))
            .and_then(MatcherFilterer::new(matcher));

        let mut feed: Feed = Faker.fake();
        // title does not match needle
        feed.title = Some("ABC".into());
        feed.website_url = Some("https://blog.ymgyt.io".into());

        assert_eq!(filter.filter(&feed), FilterResult::Use);
    }
}
