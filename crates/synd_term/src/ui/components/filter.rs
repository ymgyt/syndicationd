use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

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
    config::{Categories, Icon},
    keymap::{KeyTrie, Keymap},
    matcher::Matcher,
    types::{self, RequirementExt},
    ui::{
        self, icon,
        widgets::prompt::{Prompt, RenderCursor},
        Context,
    },
};

#[allow(dead_code)]
static LABELS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterResult {
    Use,
    Discard,
}

#[derive(Clone)]
pub(crate) struct FeedFilter {
    requirement: Requirement,
    categories: HashMap<Category<'static>, FilterCategoryState>,
    matcher: Matcher,
}

impl Default for FeedFilter {
    fn default() -> Self {
        Self {
            requirement: Filter::INITIAL_REQUIREMENT,
            categories: HashMap::new(),
            matcher: Matcher::new(),
        }
    }
}

impl FeedFilter {
    pub fn entry(&self, entry: &types::Entry) -> FilterResult {
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

    pub fn feed(&self, feed: &types::Feed) -> FilterResult {
        if !feed.requirement().is_satisfied(self.requirement) {
            return FilterResult::Discard;
        }
        if let Some(FilterCategoryState::Inactive) = self.categories.get(feed.category()) {
            return FilterResult::Discard;
        }
        if !self
            .matcher
            .r#match(feed.title.as_deref().unwrap_or_default())
        {
            return FilterResult::Discard;
        }
        FilterResult::Use
    }
}

#[derive(Debug)]
pub(crate) struct Filter {
    state: State,
    requirement: Requirement,
    categories: Vec<Category<'static>>,
    categoris_state: HashMap<Category<'static>, CategoryState>,

    prompt: Rc<RefCell<Prompt>>,
    matcher: Matcher,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    CategoryFiltering,
    SearchFiltering,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum FilterCategoryState {
    Active,
    Inactive,
}

impl FilterCategoryState {
    fn toggle(self) -> Self {
        match self {
            FilterCategoryState::Active => FilterCategoryState::Inactive,
            FilterCategoryState::Inactive => FilterCategoryState::Active,
        }
    }
}

#[derive(Debug)]
struct CategoryState {
    label: char,
    state: FilterCategoryState,
    icon: Icon,
}

impl Filter {
    const INITIAL_REQUIREMENT: Requirement = Requirement::May;

    pub fn new() -> Self {
        Self {
            state: State::Normal,
            prompt: Rc::new(RefCell::new(Prompt::new())),
            requirement: Self::INITIAL_REQUIREMENT,
            categories: Vec::new(),
            categoris_state: HashMap::new(),
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
    pub fn activate_category_filtering(&mut self) -> Keymap {
        self.state = State::CategoryFiltering;
        let mut map =
            self.categoris_state
                .iter()
                .fold(HashMap::new(), |mut map, (category, state)| {
                    let key = KeyEvent::new(KeyCode::Char(state.label), KeyModifiers::NONE);
                    let command = Command::ToggleFilterCategory {
                        category: category.clone(),
                    };
                    map.insert(key, KeyTrie::Command(command));
                    map
                });
        map.insert(
            KeyEvent::new(KeyCode::Char('+'), KeyModifiers::NONE),
            KeyTrie::Command(Command::ActivateAllFilterCategories),
        );
        map.insert(
            KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE),
            KeyTrie::Command(Command::DeactivateAllFilterCategories),
        );
        Keymap::from_map(crate::keymap::KeymapId::CategoryFiltering, map)
    }

    pub fn deactivate_filtering(&mut self) {
        self.state = State::Normal;
    }

    #[must_use]
    pub fn move_requirement(&mut self, direction: Direction) -> FeedFilter {
        self.requirement = match direction {
            Direction::Left => {
                if self.requirement == Requirement::Must {
                    Requirement::May
                } else {
                    self.requirement.up()
                }
            }
            Direction::Right => {
                if self.requirement == Requirement::May {
                    Requirement::Must
                } else {
                    self.requirement.down()
                }
            }
            _ => self.requirement,
        };

        self.feed_filter()
    }

    #[must_use]
    pub fn toggle_category_state(&mut self, category: &Category<'static>) -> FeedFilter {
        if let Some(category_state) = self.categoris_state.get_mut(category) {
            category_state.state = category_state.state.toggle();
        }

        self.feed_filter()
    }

    #[must_use]
    pub fn activate_all_categories_state(&mut self) -> FeedFilter {
        self.categoris_state
            .iter_mut()
            .for_each(|(_, state)| state.state = FilterCategoryState::Active);

        self.feed_filter()
    }

    #[must_use]
    pub fn deactivate_all_categories_state(&mut self) -> FeedFilter {
        self.categoris_state
            .iter_mut()
            .for_each(|(_, state)| state.state = FilterCategoryState::Inactive);

        self.feed_filter()
    }

    #[must_use]
    pub fn feed_filter(&self) -> FeedFilter {
        let mut matcher = self.matcher.clone();
        matcher.update_needle(self.prompt.borrow().line());
        FeedFilter {
            requirement: self.requirement,
            categories: self
                .categoris_state
                .iter()
                .map(|(c, state)| (c.clone(), state.state))
                .collect(),
            matcher,
        }
    }

    pub fn update_categories(
        &mut self,
        config: &Categories,
        populate: Populate,
        entries: &[types::Entry],
    ) {
        let new = entries
            .iter()
            .map(|entry| entry.category().clone())
            .collect::<HashSet<_>>();
        let mut prev = self.categories.drain(..).collect::<HashSet<_>>();

        let mut new_categories = match populate {
            Populate::Replace => {
                let should_remove = prev.difference(&new);
                let should_create = new.difference(&prev);

                for c in should_remove {
                    self.categoris_state.remove(c);
                }
                for c in should_create {
                    self.categoris_state.insert(
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
                    self.categoris_state.insert(
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
                self.categoris_state.get_mut(category).unwrap().label = *label;
            });
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
            Span::from("     "),
            {
                let r = self.requirement.label(&cx.theme.requirement);
                if r.content == "MAY" {
                    r.dim()
                } else {
                    r
                }
            },
            Span::from("  "),
        ];
        Line::from(spans).render(requirement_area, buf);

        let mut spans = vec![];

        for c in &self.categories {
            let state = self
                .categoris_state
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

            if self.state == State::CategoryFiltering {
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
        if self.state == State::CategoryFiltering {
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
        spans.push(Span::from("    "));

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
