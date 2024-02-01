use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
    widgets::{Block, Widget},
};

use crate::ui::{
    components::{tabs::Tab, Components},
    Context,
};

pub struct Root<'a> {
    components: &'a Components,
    cx: Context<'a>,
}

impl<'a> Root<'a> {
    pub fn new(components: &'a Components, cx: Context<'a>) -> Self {
        Self { components, cx }
    }

    fn render_browse(&self, area: Rect, buf: &mut Buffer) {
        let cx = &self.cx;

        let [tabs_area, content_area, prompt_area] = area.split(&Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]));

        self.components.tabs.render(tabs_area, buf, cx);

        match self.components.tabs.current() {
            Tab::Subscription => self.components.subscription.render(content_area, buf, cx),
            Tab::Feeds => self.components.entries.render(content_area, buf, cx),
        };

        self.components
            .prompt
            .render(prompt_area, buf, cx, &self.components.tabs.current());
    }
}

impl<'a> Widget for Root<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Background
        Block::new()
            .style(self.cx.theme.background)
            .render(area, buf);

        if self.components.auth.should_render() {
            self.components.auth.render(area, buf, &self.cx);
        } else {
            self.render_browse(area, buf);
        }
    }
}
