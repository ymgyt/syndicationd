use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
    widgets::{Block, Widget},
};

use crate::ui::{
    Context,
    components::{Components, filter::FilterContext, tabs::Tab},
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

        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [tabs_area, filter_area, content_area, prompt_area] = layout.areas(area);

        self.components.tabs.render(tabs_area, buf, cx);
        self.components.filter.render(
            filter_area,
            buf,
            &FilterContext {
                ui: cx,
                gh_options: self.components.gh_notifications.filter_options(),
            },
        );

        match cx.tab {
            Tab::Feeds => self.components.subscription.render(content_area, buf, cx),
            Tab::Entries => self.components.entries.render(content_area, buf, cx),
            Tab::GitHub => self
                .components
                .gh_notifications
                .render(content_area, buf, cx),
        };

        self.components
            .prompt
            .render(prompt_area, buf, cx, Some(self.components.tabs.current()));
    }
}

impl Widget for Root<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Background
        Block::new().style(self.cx.theme.base).render(area, buf);

        if self.components.auth.should_render() {
            let [auth_area, prompt_area] =
                Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

            self.components.auth.render(auth_area, buf, &self.cx);
            self.components
                .prompt
                .render(prompt_area, buf, &self.cx, None);
        } else {
            self.render_browse(area, buf);
        }
    }
}
