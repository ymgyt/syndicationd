use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    application::RequestId,
    ui::widgets::throbber::{throbber, Throbber, ThrobberState},
    ui::{widgets::throbber::throbber::WhichUse, Context},
};

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
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>, tab: Option<Tab>) {
        match self.error_message.as_ref() {
            Some(error_message) => Self::render_error(area, buf, cx, error_message),
            None => Self::render_prompt(area, buf, cx, tab),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn render_prompt(area: Rect, buf: &mut Buffer, cx: &Context<'_>, tab: Option<Tab>) {
        let keys = &[("q", ""), ("Tab", "󰹳"), ("j/k", "󰹹"), ("r", "󰑓")][..];
        let per_screen_keys = match tab {
            Some(Tab::Feeds) => [("a", "󰑫"), ("d", "󰼡"), ("Ent", "󰏌")].iter().chain(keys),
            Some(Tab::Entries) => [("Ent", "󰏌")].iter().chain(keys),
            // Imply login
            None => [("q", ""), ("j/k", "󰹹")].iter().chain(&[("Ent", "󰏌")][..]),
        };

        let spans = per_screen_keys
            .flat_map(|(key, desc)| {
                // let key = Span::styled(format!(" {key}"), cx.theme.prompt.key);
                let desc = Span::styled(format!(" {key} {desc} "), cx.theme.prompt.key_desc);
                let sep = Span::styled("", cx.theme.prompt.key);
                [desc, sep]
            })
            .collect::<Vec<_>>();

        let area = {
            if let Some(in_flight) = cx.in_flight.recent_in_flight() {
                let label = match in_flight {
                    RequestId::FetchEntries => "Fetch entries...",
                    RequestId::FetchSubscription => "Fetch subscription...",
                    RequestId::SubscribeFeed => "Subscribe feed...",
                    RequestId::UnsubscribeFeed => "Unsubscribe feed...",
                };
                let horizontal = Layout::horizontal([
                    Constraint::Length(label.len() as u16 + 1),
                    Constraint::Fill(1),
                ]);
                let [in_flight_area, area] = horizontal.areas(area);

                let mut throbber_state = ThrobberState::default();
                throbber_state.calc_step(cx.in_flight.throbber_step());

                let throbber = Throbber::default()
                    .label(label)
                    .throbber_set(throbber::BRAILLE_EIGHT_DOUBLE)
                    .use_type(WhichUse::Spin);

                throbber.render(in_flight_area, buf, &mut throbber_state);
                area
            } else {
                area
            }
        };

        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center)
            .style(cx.theme.prompt.background)
            .render(area, buf);
    }

    fn render_error(area: Rect, buf: &mut Buffer, cx: &Context<'_>, error_message: &str) {
        let line = Line::from(error_message);
        Paragraph::new(line)
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .style(cx.theme.error.message)
            .render(area, buf);
    }
}
