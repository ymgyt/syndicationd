use std::{cell::RefCell, rc::Rc};

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
    client::gh::{FetchNotificationInclude, FetchNotificationParticipating},
    command::{FilterTarget, PromptAction},
    config::Categories,
    keymap,
    matcher::Matcher,
    types::{
        RequirementExt,
        gh::{PullRequestState, Reason, RepoVisibility},
    },
    ui::{
        Context, icon,
        widgets::prompt::{Prompt, RenderCursor},
        widgets::{
            filter::{
                category::{CategoriesState, FilterCategoryState},
                feed::RequirementFilterer,
                gh::GhNotificationHandler,
            },
            gh_notifications::GhNotificationFilterOptions,
        },
    },
};

mod feed;
pub(crate) use feed::{FeedFilterer, FeedHandler};

mod gh;

mod category;
pub(crate) use category::CategoryFilterer;

mod composed;
pub(crate) use composed::{Composable, ComposedFilterer};

mod matcher;
pub(crate) use matcher::MatcherFilterer;

pub(crate) type CategoryAndMatcherFilterer = ComposedFilterer<CategoryFilterer, MatcherFilterer>;

/// The selected categories and search text when the filter changed.
#[derive(Debug)]
pub(crate) struct FilterCriteria {
    categories: CategoryFilterer,
    query: String,
}

impl FilterCriteria {
    pub(crate) fn filterer(&self, matcher: &mut Matcher) -> CategoryAndMatcherFilterer {
        matcher.update_needle(&self.query);
        self.categories
            .clone()
            .and_then(MatcherFilterer::new(matcher.clone()))
    }

    pub(crate) fn feed_filterer(
        &self,
        requirement: Requirement,
        matcher: &mut Matcher,
    ) -> FeedFilterer {
        matcher.update_needle(&self.query);
        RequirementFilterer::new(requirement)
            .and_then(self.categories.clone())
            .and_then(MatcherFilterer::new(matcher.clone()))
    }
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
pub(crate) struct FilterWidget {
    pub(crate) target: FilterTarget,
    categories: Rc<Categories>,
    state: State,
    keymap_dirty: bool,
    feed: FeedHandler,
    gh_notification: GhNotificationHandler,

    prompt: Rc<RefCell<Prompt>>,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    CategoryFiltering(FilterTarget),
    SearchFiltering,
}

impl FilterWidget {
    pub fn new(categories: Rc<Categories>, target: FilterTarget) -> Self {
        Self {
            categories,
            state: State::Normal,
            target,
            keymap_dirty: true,
            prompt: Rc::new(RefCell::new(Prompt::new())),
            feed: FeedHandler::new(),
            gh_notification: GhNotificationHandler::new(),
        }
    }

    fn set_state(&mut self, state: State) {
        if self.state != state {
            self.state = state;
            self.keymap_dirty = true;
        }
    }

    pub(crate) fn install_filter_keymap(&mut self, map: &mut keymap::Keymap) {
        if !std::mem::take(&mut self.keymap_dirty) {
            return;
        }
        map.reset_layer(keymap::Layer::CategoryFilter);
        if let Some(categories) = self.category_filter_keymap() {
            map.set_layer(keymap::Layer::CategoryFilter, categories);
        }
    }

    pub fn activate_search_filtering(&mut self) {
        self.set_state(State::SearchFiltering);
    }

    pub fn is_search_active(&self) -> bool {
        self.state == State::SearchFiltering
    }

    pub fn is_category_filtering_active(&self) -> bool {
        self.category_filter_target().is_some()
    }

    pub(crate) fn category_filter_target(&self) -> Option<FilterTarget> {
        match self.state {
            State::CategoryFiltering(target) => Some(target),
            State::Normal | State::SearchFiltering => None,
        }
    }

    pub(crate) fn category_filter_keymap(&self) -> Option<keymap::LayerKeymap> {
        let State::CategoryFiltering(target) = self.state else {
            return None;
        };

        let mut map = keymap::LayerKeymap::default();
        let categories = self.categories_state(target);
        for category in &categories.categories {
            let state = &categories.state[category];
            if state.label != ' ' {
                map.bind(tuiframework::keymap::Binding {
                    on: keymap::KeySequence::new(vec![keymap::KeyStroke::from_char(state.label)])
                        .unwrap(),
                    command: keymap::FilterAction::ToggleCategory {
                        target,
                        category: category.clone(),
                    }
                    .into(),
                    desc: Some(format!("Toggle {category} category")),
                })
                .expect("valid category binding");
            }
        }
        for (key, action, desc) in [
            (
                "+",
                keymap::FilterAction::ActivateAllCategories { target },
                "Activate all categories",
            ),
            (
                "-",
                keymap::FilterAction::DeactivateAllCategories { target },
                "Deactivate all categories",
            ),
        ] {
            map.bind(tuiframework::keymap::Binding {
                on: key.parse().unwrap(),
                command: action.into(),
                desc: Some(desc.into()),
            })
            .expect("valid category binding");
        }
        Some(map)
    }

    pub fn activate_category_filtering(&mut self, target: FilterTarget) {
        self.set_state(State::CategoryFiltering(target));
    }

    fn categories_state(&self, target: FilterTarget) -> &CategoriesState {
        match target {
            FilterTarget::Feeds => &self.feed.categories_state,
            FilterTarget::GhNotifications => &self.gh_notification.categories_state,
        }
    }

    fn categories_state_mut(&mut self, target: FilterTarget) -> &mut CategoriesState {
        match target {
            FilterTarget::Feeds => &mut self.feed.categories_state,
            FilterTarget::GhNotifications => &mut self.gh_notification.categories_state,
        }
    }

    pub fn deactivate_filtering(&mut self) {
        self.set_state(State::Normal);
    }

    pub(crate) fn edit_prompt(&mut self, action: &PromptAction) -> bool {
        self.prompt.borrow_mut().apply(action)
    }

    #[must_use]
    pub fn move_requirement(&mut self, direction: Direction) -> bool {
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
            _ => return false,
        };

        true
    }

    #[must_use]
    pub fn toggle_category_state(
        &mut self,
        category: &Category<'static>,
        target: FilterTarget,
    ) -> bool {
        let Some(category) = self.categories_state_mut(target).state.get_mut(category) else {
            return false;
        };
        category.state = category.state.toggle();
        true
    }

    pub(crate) fn set_all_categories_active(&mut self, target: FilterTarget, active: bool) -> bool {
        let state = if active {
            FilterCategoryState::Active
        } else {
            FilterCategoryState::Inactive
        };
        let mut changed = false;
        for category in self.categories_state_mut(target).state.values_mut() {
            changed |= category.state != state;
            category.state = state;
        }
        changed
    }

    pub(crate) fn changed(&self, target: FilterTarget) -> crate::event::Event {
        let criteria = FilterCriteria {
            categories: CategoryFilterer::new(
                self.categories_state(target)
                    .state
                    .iter()
                    .map(|(category, state)| (category.clone(), state.state))
                    .collect(),
            ),
            query: self.prompt.borrow().line().to_owned(),
        };
        match target {
            FilterTarget::Feeds => crate::event::Event::FeedFilterChanged {
                requirement: self.feed.requirement,
                criteria,
            },
            FilterTarget::GhNotifications => crate::event::Event::GhFilterChanged(criteria),
        }
    }

    pub(crate) fn update_categories(
        &mut self,
        target: FilterTarget,
        populate: Populate,
        categories: &[Category<'static>],
    ) {
        let state = match target {
            FilterTarget::Feeds => &mut self.feed.categories_state,
            FilterTarget::GhNotifications => &mut self.gh_notification.categories_state,
        };
        state.update(&self.categories, populate, categories.iter().cloned());
        self.keymap_dirty |= self.category_filter_target() == Some(target);
    }
}

pub(crate) struct FilterContext<'a> {
    pub(crate) ui: &'a Context<'a>,
    pub(crate) gh_options: &'a GhNotificationFilterOptions,
    pub(crate) target: FilterTarget,
}

impl FilterWidget {
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer, cx: &FilterContext<'_>) {
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
        self.render_search(search_area, buf, cx.ui, cx.target);
    }

    #[allow(unstable_name_collisions)]
    fn render_filter(&self, area: Rect, buf: &mut Buffer, cx: &FilterContext<'_>) {
        let mut spans = vec![Span::from(concat!(icon!(filter), " Filter")).dim()];

        match cx.target {
            FilterTarget::Feeds => {
                let mut r = self.feed.requirement.label(&cx.ui.theme.requirement);
                if r.content == "MAY" {
                    r = r.dim();
                }
                spans.extend([Span::from("    "), r, Span::from("  ")]);
            }
            FilterTarget::GhNotifications => {
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

        let (categories, categories_state) = match cx.target {
            FilterTarget::Feeds => (
                &self.feed.categories_state.categories,
                &self.feed.categories_state.state,
            ),
            FilterTarget::GhNotifications => (
                &self.gh_notification.categories_state.categories,
                &self.gh_notification.categories_state.state,
            ),
        };

        let mut spans = vec![];

        let is_active =
            matches!(self.state, State::CategoryFiltering(active) if active == cx.target);
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

    fn render_search(&self, area: Rect, buf: &mut Buffer, _cx: &Context<'_>, target: FilterTarget) {
        let mut spans = vec![];
        let mut label = Span::from(concat!(icon!(search), " Search"));
        if self.state != State::SearchFiltering {
            label = label.dim();
        }
        spans.push(label);
        {
            let padding = match target {
                FilterTarget::Feeds => "   ",
                FilterTarget::GhNotifications => " ",
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
    use std::collections::HashMap;

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
