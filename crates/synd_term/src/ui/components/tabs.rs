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

impl Tab {
    fn width(self) -> u16 {
        match self {
            Tab::Entries => 7,
            Tab::Feeds => 5,
            Tab::GitHub => 6,
        }
    }
}

pub struct Tabs {
    pub selected: usize,
    pub tabs: Vec<Tab>,
}

impl Tabs {
    const PADDING: &'static str = "    ";

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

    fn width(&self) -> u16 {
        #[allow(clippy::cast_possible_truncation)]
        self.tabs.iter().fold(0, |width, tab| {
            width + tab.width() + (Self::PADDING.len() as u16) + 2
        })
    }
}

impl Tabs {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let area = Rect {
            x: area.x + 2,
            width: area.width.saturating_sub(3),
            ..area
        };

        let horizontal = Layout::horizontal([Constraint::Min(0), Constraint::Length(self.width())]);
        let [title, tabs] = horizontal.areas(area);

        Paragraph::new(Span::styled("Syndicationd", cx.theme.application_title)).render(title, buf);

        TuiTabs::new(self.tabs.iter().map(|tab| match tab {
            Tab::Entries => concat!(icon!(entries), " Entries"),
            Tab::Feeds => concat!(icon!(feeds), " Feeds"),
            Tab::GitHub => concat!(icon!(github), " GitHub"),
        }))
        .style(cx.theme.tabs)
        .divider("")
        .padding(Self::PADDING, "")
        .select(self.selected)
        .highlight_style(cx.theme.tabs_selected)
        .render(tabs, buf);
    }
}
