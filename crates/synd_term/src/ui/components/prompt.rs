use ratatui::{
    prelude::{Alignment, Buffer, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::ui::Context;

use super::tabs::Tab;

pub struct Prompt {
    error_message: Option<String>,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            error_message: None,
        }
    }

    pub fn set_error_message(&mut self, msg: String) {
        self.error_message = Some(msg);
    }

    pub fn clear_error_message(&mut self) {
        self.error_message = None;
    }
}

impl Prompt {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>, tab: &Tab) {
        // If has error message, render it
        let paragraph = if let Some(error_message) = self.error_message.as_ref() {
            let line = Line::styled(error_message, cx.theme.error.message);
            Paragraph::new(line)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true })
                .style(cx.theme.prompt.background)
        } else {
            let keys = [("q", "Quit"), ("Tab", "Next Tab"), ("j/k", "Up/Down")];
            let per_screen_keys = match tab {
                Tab::Subscription => [
                    ("a", "Subscribe"),
                    ("d", "Unsubscribe"),
                    ("Enter", "Open Feed"),
                ]
                .iter(),
                Tab::Feeds => [].iter(),
            };

            let spans = keys
                .iter()
                .chain(per_screen_keys)
                .flat_map(|(key, desc)| {
                    let key = Span::styled(format!(" {key} "), cx.theme.prompt.key);
                    let desc = Span::styled(format!(" {desc} "), cx.theme.prompt.key_desc);
                    [key, desc]
                })
                .collect::<Vec<_>>();
            Paragraph::new(Line::from(spans))
                .alignment(Alignment::Center)
                .style(cx.theme.prompt.background)
        };

        paragraph.render(area, buf);
    }
}
