use std::collections::{HashMap, HashSet};

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
    config::{Categories, Icon},
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

pub struct FeedFilter {
    requirement: Requirement,
}

impl Default for FeedFilter {
    fn default() -> Self {
        Self {
            requirement: Filter::INTIAL_REQUIREMENT,
        }
    }
}

impl FeedFilter {
    pub fn entry(&self, entry: &types::Entry) -> FilterResult {
        if !entry.requirement().is_satisfied(self.requirement) {
            return FilterResult::Discard;
        }
        FilterResult::Use
    }

    pub fn feed(&self, feed: &types::Feed) -> FilterResult {
        if !feed.requirement().is_satisfied(self.requirement) {
            return FilterResult::Discard;
        }
        FilterResult::Use
    }
}

#[derive(Debug)]
pub struct Filter {
    requirement: Requirement,
    categories: Vec<Category<'static>>,
    categoris_state: HashMap<Category<'static>, CategoryState>,
}

#[derive(Debug)]
struct CategoryState {
    is_active: bool,
    icon: Icon,
}

impl Filter {
    const INTIAL_REQUIREMENT: Requirement = Requirement::May;

    pub fn new() -> Self {
        Self {
            requirement: Self::INTIAL_REQUIREMENT,
            categories: Vec::new(),
            categoris_state: HashMap::new(),
        }
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

        FeedFilter {
            requirement: self.requirement,
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

        match action {
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
                            is_active: true,
                            icon: config.icon(c).unwrap_or_else(|| ui::default_icon()).clone(),
                        },
                    );
                }

                let mut new_categories = new.into_iter().collect::<Vec<_>>();
                new_categories.sort_unstable();

                self.categories = new_categories;
            }
            ListAction::Append => {
                let should_create = new.difference(&prev);
                for c in should_create {
                    self.categoris_state.insert(
                        c.clone(),
                        CategoryState {
                            is_active: true,
                            icon: config.icon(c).unwrap_or_else(|| ui::default_icon()).clone(),
                        },
                    );
                }

                let mut new_categories = prev.into_iter().collect::<Vec<_>>();
                new_categories.extend(new);
                new_categories.sort_unstable();
                self.categories = new_categories;
            }
        }
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
            Span::from("Requirement ").dim(),
        ];
        spans.extend(self.requirement.label(cx.theme.requiment_fg));

        spans.extend([Span::from(" "), Span::from(" Category").dim()]);

        for c in &self.categories {
            let state = self
                .categoris_state
                .get(c)
                .expect("CategoryState is not found. THIS IS A BUG");
            let mut s = Span::from(state.icon.symbol());
            if let Some(fg) = state.icon.color() {
                s = s.fg(fg);
            }
            if !state.is_active {
                s = s.dim();
            }

            spans.extend([Span::from("  "), s]);
        }

        let filter = Line::from(spans);
        filter.render(area, buf);
    }
}
