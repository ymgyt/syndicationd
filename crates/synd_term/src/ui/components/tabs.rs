use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
    text::Span,
    widgets::{Paragraph, Tabs as TuiTabs, Widget},
};

use crate::{
    application::{Direction, Features, IndexOutOfRange},
    ui::{icon, Context},
};

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Tab {
    Entries,
    Feeds,
    GitHub,
}

pub struct Tabs {
    pub selected: usize,
    pub tabs: Vec<Tab>,
}

impl Tabs {
    pub fn new(features: &'_ Features) -> Self {
        let mut tabs = vec![Tab::Entries, Tab::Feeds];
        if features.enable_github_notification {
            tabs.insert(0, Tab::GitHub);
        }
        Self { selected: 0, tabs }
    }

    pub fn current(&self) -> Tab {
        self.tabs[self.selected]
    }

    pub fn move_selection(&mut self, direction: Direction) -> Tab {
        self.selected = direction.apply(self.selected, self.tabs.len(), IndexOutOfRange::Wrapping);
        self.current()
    }
}

impl Tabs {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let area = Rect {
            x: area.x + 2,
            width: area.width.saturating_sub(3),
            ..area
        };

        // TODO: query length to tabs
        let horizontal = Layout::horizontal([Constraint::Min(0), Constraint::Length(36)]);
        let [title, tabs] = horizontal.areas(area);

        Paragraph::new(Span::styled("Syndicationd", cx.theme.application_title)).render(title, buf);

        TuiTabs::new(self.tabs.iter().map(|tab| match tab {
            Tab::Entries => concat!(icon!(entries), " Entries"),
            Tab::Feeds => concat!(icon!(feeds), " Feeds"),
            Tab::GitHub => concat!(icon!(github), " GitHub"),
        }))
        .style(cx.theme.tabs)
        .divider("")
        .padding("    ", "")
        .select(self.selected)
        .highlight_style(cx.theme.tabs_selected)
        .render(tabs, buf);
    }
}
