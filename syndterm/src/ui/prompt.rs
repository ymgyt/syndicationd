use ratatui::{
    prelude::{Alignment, Buffer, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::ui::Context;

pub struct Prompt {}

impl Prompt {
    pub fn new() -> Self {
        Self {}
    }
}

impl Prompt {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
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
            .render(area, buf);
    }
}
