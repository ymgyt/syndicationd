use ratatui::{
    prelude::{Buffer, Constraint, Layout, Rect},
    widgets::{Block, Widget},
};

use crate::{
    application::component::Components,
    ui::{
        Context,
        widgets::{authentication::AuthWidget, filter::FilterContext, tabs::Tab},
    },
};

pub struct AppWidget<'a> {
    components: &'a Components,
    cx: Context<'a>,
}

impl<'a> AppWidget<'a> {
    pub fn new(components: &'a Components, cx: Context<'a>) -> Self {
        Self { components, cx }
    }

    fn render_browse(&self, area: Rect, buf: &mut Buffer) {
        let cx = &self.cx;
        let shell = &self.components.shell;
        let feeds = &self.components.feeds;
        let gh = &self.components.gh;

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
                gh_options: gh.notifications.filter_options(),
                target: shell.current_filter_target(),
            },
        );

        match cx.tab {
            Tab::Feeds => feeds.subscription.render(content_area, buf, cx),
            Tab::Entries => feeds.entries.render(content_area, buf, cx),
            Tab::Gh => gh.notifications.render(content_area, buf, cx),
        }

        shell
            .prompt
            .render(prompt_area, buf, cx, Some(shell.tabs.current()));
    }
}

impl Widget for AppWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        Block::new().style(self.cx.theme.base).render(area, buf);

        match AuthWidget::from_shell(&self.components.shell, self.cx.theme) {
            Some(auth) => {
                let [auth_area, prompt_area] =
                    Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

                auth.render(auth_area, buf);
                self.components
                    .shell
                    .prompt
                    .render(prompt_area, buf, &self.cx, None);
            }
            None => self.render_browse(area, buf),
        }
    }
}
