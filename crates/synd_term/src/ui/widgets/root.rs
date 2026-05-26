use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
    widgets::{Block, Widget},
};

use crate::{
    application::component::AppComponent,
    ui::{
        Context,
        widgets::{filter::FilterContext, tabs::Tab},
    },
};

pub struct AppWidget<'a> {
    components: &'a AppComponent,
    cx: Context<'a>,
}

impl<'a> AppWidget<'a> {
    pub fn new(components: &'a AppComponent, cx: Context<'a>) -> Self {
        Self { components, cx }
    }

    fn render_browse(&self, area: Rect, buf: &mut Buffer) {
        let cx = &self.cx;
        let shell = &self.components.shell;
        let feeds = &self.components.feeds;
        let github = &self.components.github;

        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [tabs_area, filter_area, content_area, prompt_area] = layout.areas(area);

        shell.tabs.render(tabs_area, buf, cx);
        shell.filter.render(
            filter_area,
            buf,
            &FilterContext {
                ui: cx,
                gh_options: github.notifications.filter_options(),
            },
        );

        match cx.tab {
            Tab::Feeds => feeds.subscription.render(content_area, buf, cx),
            Tab::Entries => feeds.entries.render(content_area, buf, cx),
            Tab::GitHub => github.notifications.render(content_area, buf, cx),
        }

        shell
            .prompt
            .render(prompt_area, buf, cx, Some(shell.tabs.current()));
    }
}

impl Widget for AppWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Background
        Block::new().style(self.cx.theme.base).render(area, buf);

        if self.components.shell.auth.should_render() {
            let [auth_area, prompt_area] =
                Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

            self.components.shell.auth.render(auth_area, buf, &self.cx);
            self.components
                .shell
                .prompt
                .render(prompt_area, buf, &self.cx, None);
        } else {
            self.render_browse(area, buf);
        }
    }
}
