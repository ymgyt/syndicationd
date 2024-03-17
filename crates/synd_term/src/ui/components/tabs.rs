use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
    text::Span,
    widgets::{Paragraph, Tabs as TuiTabs, Widget},
};

use crate::{
    application::{Direction, IndexOutOfRange},
    ui::Context,
};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Tab {
    Entries,
    Feeds,
}

pub struct Tabs {
    pub selected: usize,
    pub tabs: Vec<&'static str>,
}

impl Tabs {
    pub fn new() -> Self {
        Self {
            selected: 0,
            tabs: vec!["󱉯 Entries", "󰑫 Feeds"],
        }
    }

    pub fn current(&self) -> Tab {
        match self.selected {
            0 => Tab::Entries,
            1 => Tab::Feeds,
            _ => unreachable!(),
        }
    }

    pub fn move_selection(&mut self, direction: &Direction) -> Tab {
        self.selected = direction.apply(self.selected, self.tabs.len(), IndexOutOfRange::Wrapping);
        self.current()
    }
}

impl Tabs {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let area = Rect {
            x: area.x + 4,
            width: area.width - 6,
            ..area
        };
        // left padding * 2 + len("Entries" + "Feeds") = 20
        let horizontal = Layout::horizontal([Constraint::Min(0), Constraint::Length(24)]);
        let [title, tabs] = horizontal.areas(area);

        Paragraph::new(Span::styled("Syndicationd", cx.theme.application_title)).render(title, buf);

        TuiTabs::new(self.tabs.clone())
            .style(cx.theme.tabs)
            .divider("")
            .padding("    ", "")
            .select(self.selected)
            .highlight_style(cx.theme.tabs_selected)
            .render(tabs, buf);
    }
}
