use std::{cell::RefCell, rc::Rc};

use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Padding, Widget},
};
use synd_client::payload;
use synd_feed::types::{Category, Requirement};

use crate::{
    application::{Direction, Populate},
    client::gh::{FetchNotificationInclude, FetchNotificationParticipating},
    command::FilterTarget,
    config::Categories,
    keymap,
    matcher::Matcher,
    types::{
        EntryExt, RequirementExt,
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
pub(crate) struct FilterWidget {
    state: State,
    feed: FeedHandler,
    gh_notification: GhNotificationHandler,

    prompt: Rc<RefCell<Prompt>>,
    matcher: Matcher,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    CategoryFiltering(FilterTarget),
    SearchFiltering,
}

impl FilterWidget {
    pub fn new() -> Self {
        Self {
            state: State::Normal,
            prompt: Rc::new(RefCell::new(Prompt::new())),
            feed: FeedHandler::new(),
            gh_notification: GhNotificationHandler::new(),
            matcher: Matcher::new(),
        }
    }

    pub fn activate_search_filtering(&mut self) {
        self.state = State::SearchFiltering;
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

    pub(crate) fn is_filtering_active(&self) -> bool {
        self.state != State::Normal
    }

    pub(crate) fn category_filter_keymap(&self) -> Option<keymap::LayerKeymap> {
        let State::CategoryFiltering(target) = self.state else {
            return None;
        };

        let mut keymap = keymap::LayerKeymap::builder(keymap::Layer::CategoryFilter);
        for (category, state) in &self.categories_state(target).state {
            if state.label == ' ' {
                continue;
            }
            keymap
                .bind_key(
                    keymap::KeyStroke::from_char(state.label),
                    keymap::KeymapAction::Filter(keymap::FilterAction::ToggleCategory {
                        target,
                        category: category.clone(),
                    }),
                    Some(format!("Toggle {category} category")),
                )
                .expect("valid category filter key binding");
        }
        keymap
            .bind(
                ["+"],
                keymap::KeymapAction::Filter(keymap::FilterAction::ActivateAllCategories {
                    target,
                }),
                Some("Activate all categories"),
            )
            .expect("valid category filter key binding");
        keymap
            .bind(
                ["-"],
                keymap::KeymapAction::Filter(keymap::FilterAction::DeactivateAllCategories {
                    target,
                }),
                Some("Deactivate all categories"),
            )
            .expect("valid category filter key binding");

        Some(keymap.build().expect("valid category filter keymap"))
    }

    pub fn activate_category_filtering(&mut self, target: FilterTarget) {
        self.state = State::CategoryFiltering(target);
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
        self.state = State::Normal;
    }

    pub(crate) fn insert_prompt_char(&mut self, ch: char) {
        self.prompt.borrow_mut().insert_char(ch);
    }

    pub(crate) fn delete_prompt_backward(&mut self) {
        self.prompt.borrow_mut().delete_backward();
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
        target: FilterTarget,
    ) -> Filterer {
        if let Some(category_state) = self.categories_state_mut(target).state.get_mut(category) {
            category_state.state = category_state.state.toggle();
        }

        self.filterer(target)
    }

    #[must_use]
    pub fn activate_all_categories_state(&mut self, target: FilterTarget) -> Filterer {
        self.categories_state_mut(target)
            .state
            .iter_mut()
            .for_each(|(_, state)| state.state = FilterCategoryState::Active);

        self.filterer(target)
    }

    #[must_use]
    pub fn deactivate_all_categories_state(&mut self, target: FilterTarget) -> Filterer {
        self.categories_state_mut(target)
            .state
            .iter_mut()
            .for_each(|(_, state)| state.state = FilterCategoryState::Inactive);

        self.filterer(target)
    }

    #[must_use]
    pub(crate) fn filterer(&self, target: FilterTarget) -> Filterer {
        match target {
            FilterTarget::Feeds => Filterer::Feed(self.feed_filterer()),
            FilterTarget::GhNotifications => {
                Filterer::GhNotification(self.gh_notification_filterer())
            }
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

    pub fn update_categories<'a>(
        &mut self,
        config: &Categories,
        populate: Populate,
        entries: impl IntoIterator<Item = &'a payload::Entry>,
    ) {
        self.feed.categories_state.update(
            config,
            populate,
            entries.into_iter().map(EntryExt::category).cloned(),
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
    pub(super) target: FilterTarget,
}

impl FilterWidget {
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
