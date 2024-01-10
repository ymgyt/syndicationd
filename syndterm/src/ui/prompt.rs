use ratatui::{
    prelude::{Alignment, Buffer, Rect},
    text::{Line, Span},
    widgets::{Paragraph, Widget, Wrap},
};

use crate::ui::Context;

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
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        // If has error message, render it
        let paragraph = if let Some(error_message) = self.error_message.as_ref() {
            let line = Line::styled(error_message, cx.theme.error.message);
            Paragraph::new(line)
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true })
                .style(cx.theme.prompt.background)
        } else {
            let keys = [
                ("q", "Quit"),
                ("Tab", "Next Tab"),
                ("a", "Add Subscription"),
            ];
            let spans = keys
                .iter()
                .flat_map(|(key, desc)| {
                    let key = Span::styled(format!(" {} ", key), cx.theme.prompt.key);
                    let desc = Span::styled(format!(" {} ", desc), cx.theme.prompt.key_desc);
                    [key, desc]
                })
                .collect::<Vec<_>>();
            Paragraph::new(Line::from(spans))
                .alignment(Alignment::Center)
                .style(cx.theme.prompt.background)
        };

        paragraph.render(area, buf)
    }
}
