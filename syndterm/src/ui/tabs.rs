use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
    text::Span,
    widgets::{Paragraph, Tabs as TuiTabs, Widget},
};

use crate::{application::Direction, ui::Context};

pub struct Tabs {
    pub selected: usize,
    pub tabs: Vec<&'static str>,
}

impl Tabs {
    pub fn new() -> Self {
        Self {
            selected: 0,
            tabs: vec![" Feeds ", " Subscription "],
        }
    }

    pub fn move_selection(&mut self, direction: Direction) {
        let selected = match direction {
            Direction::Left => {
                if self.selected == 0 {
                    self.tabs.len() - 1
                } else {
                    self.selected - 1
                }
            }
            Direction::Right => (self.selected + 1) % self.tabs.len(),
            _ => self.selected,
        };
        self.selected = selected;
    }
}

impl Tabs {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let horizontal = Layout::horizontal([Constraint::Min(0), Constraint::Length(25)]);
        let [title, tabs] = area.split(&horizontal);

        Paragraph::new(Span::styled("Syndicationd", cx.theme.application_title)).render(title, buf);

        TuiTabs::new(self.tabs.clone())
            .style(cx.theme.tabs)
            .divider("")
            .select(cx.state.tabs.selected)
            .highlight_style(cx.theme.tabs_selected)
            .render(tabs, buf);
    }
}
