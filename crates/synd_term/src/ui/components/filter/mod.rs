use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    command::Command,
    config::Categories,
    keymap::{KeyTrie, Keymap},
    matcher::Matcher,
    types::{self, RequirementExt},
    ui::{
        components::{
            filter::{
                category::{CategoriesState, FilterCategoryState},
                github::GhNotificationHandler,
            },
            tabs::Tab,
        },
        icon,
        widgets::prompt::{Prompt, RenderCursor},
        Context,
    },
};

mod feed;
pub(crate) use feed::{FeedFilterer, FeedHandler};

mod github;
pub(crate) use github::GhNotificationFilterer;

mod category;

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

#[derive(Clone, Debug)]
pub(crate) enum Filterer {
    Feed(FeedFilterer),
    GhNotification(GhNotificationFilterer),
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
        let mut matcher = self.matcher.clone();
        matcher.update_needle(self.prompt.borrow().line());
        FeedFilterer {
            requirement: self.feed.requirement,
            categories: self
                .feed
                .categories_state
                .state
                .iter()
                .map(|(c, state)| (c.clone(), state.state))
                .collect(),
            matcher,
        }
    }

    #[must_use]
    fn gh_notification_filterer(&self) -> GhNotificationFilterer {
        GhNotificationFilterer {
            categories: self
                .gh_notification
                .categories_state
                .state
                .iter()
                .map(|(c, state)| (c.clone(), state.state))
                .collect(),
        }
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

impl Filter {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
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

        self.render_filter(filter_area, buf, cx);
        self.render_search(search_area, buf, cx);
    }

    fn render_filter(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let horizontal = Layout::horizontal([Constraint::Length(18), Constraint::Fill(1)]);
        let [requirement_area, categories_area] = horizontal.areas(area);

        let spans = vec![
            Span::from(concat!(icon!(filter), " Filter")).dim(),
            Span::from("    "),
            {
                let r = self.feed.requirement.label(&cx.theme.requirement);
                if r.content == "MAY" {
                    r.dim()
                } else {
                    r
                }
            },
            Span::from("  "),
        ];
        Line::from(spans).render(requirement_area, buf);

        let lane = cx.tab.into();
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

    fn render_search(&self, area: Rect, buf: &mut Buffer, _cx: &Context<'_>) {
        let mut spans = vec![];
        let mut label = Span::from(concat!(icon!(search), " Search"));
        if self.state != State::SearchFiltering {
            label = label.dim();
        }
        spans.push(label);
        spans.push(Span::from("   "));

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
        let filter = FeedFilterer {
            requirement: Requirement::May,
            categories: HashMap::new(),
            matcher,
        };

        let mut feed: Feed = Faker.fake();
        // title does not match needle
        feed.title = Some("ABC".into());
        feed.website_url = Some("https://blog.ymgyt.io".into());

        assert_eq!(filter.filter_feed(&feed), FilterResult::Use);
    }
}
