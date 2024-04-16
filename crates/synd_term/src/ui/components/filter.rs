use std::collections::{HashMap, HashSet};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    text::{Line, Span},
    widgets::{Block, Padding, Widget},
};
use synd_feed::types::{Category, Requirement};

use crate::{
    application::{Direction, ListAction},
    command::Command,
    config::{Categories, Icon},
    keymap::{KeyTrie, Keymap},
    types::{self, RequirementExt},
    ui::{self, icon, Context},
};

#[allow(dead_code)]
static LABELS: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's',
    't', 'u', 'v', 'w', 'x', 'y', 'z',
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FilterResult {
    Use,
    Discard,
}

#[derive(Clone)]
pub struct FeedFilter {
    requirement: Requirement,
    categories: HashMap<Category<'static>, FilterCategoryState>,
}

impl Default for FeedFilter {
    fn default() -> Self {
        Self {
            requirement: Filter::INITIAL_REQUIREMENT,
            categories: HashMap::new(),
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
        FilterResult::Use
    }

    pub fn feed(&self, feed: &types::Feed) -> FilterResult {
        if !feed.requirement().is_satisfied(self.requirement) {
            return FilterResult::Discard;
        }
        if let Some(FilterCategoryState::Inactive) = self.categories.get(feed.category()) {
            return FilterResult::Discard;
        }
        FilterResult::Use
    }
}

#[derive(Debug)]
pub struct Filter {
    state: State,
    requirement: Requirement,
    categories: Vec<Category<'static>>,
    categoris_state: HashMap<Category<'static>, CategoryState>,
}

#[derive(Debug, PartialEq, Eq)]
enum State {
    Normal,
    CategoryFiltering,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FilterCategoryState {
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
            requirement: Self::INITIAL_REQUIREMENT,
            categories: Vec::new(),
            categoris_state: HashMap::new(),
        }
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

    pub fn deactivate_category_filtering(&mut self) {
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

    fn feed_filter(&self) -> FeedFilter {
        FeedFilter {
            requirement: self.requirement,
            categories: self
                .categoris_state
                .iter()
                .map(|(c, state)| (c.clone(), state.state))
                .collect(),
        }
    }

    pub fn update_categories(
        &mut self,
        config: &Categories,
        action: ListAction,
        entries: &[types::Entry],
    ) {
        let new = entries
            .iter()
            .map(|entry| entry.category().clone())
            .collect::<HashSet<_>>();
        let prev = self.categories.drain(..).collect::<HashSet<_>>();

        let mut new_categories = match action {
            ListAction::Replace => {
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
            ListAction::Append => {
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

                let mut new_categories = prev.into_iter().collect::<Vec<_>>();
                new_categories.extend(new);
                new_categories
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
                left: 3,
                right: 1,
                top: 0,
                bottom: 0,
            })
            .inner(area);
        let mut spans = vec![
            Span::from(concat!(icon!(filter), " Filter")),
            Span::from("     "),
            Span::from(concat!(icon!(requirement), " Requirement")).dim(),
            Span::from(" "),
        ];
        spans.extend(self.requirement.label(&cx.theme.requirement));

        spans.extend([
            Span::from(" "),
            Span::from(concat!(icon!(category), " Categories")).dim(),
            Span::from("  "),
        ]);

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

        let filter = Line::from(spans);
        filter.render(area, buf);
    }
}
