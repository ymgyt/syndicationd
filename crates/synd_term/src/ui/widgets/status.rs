use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    application::{InFlightStatus, RequestProgress},
    keymap::KeyHint,
    ui::{
        Context,
        widgets::throbber::{
            Throbber, ThrobberState,
            throbber::{self, WhichUse},
        },
    },
};

pub struct StatusLineWidget {
    error_message: Option<String>,
}

impl StatusLineWidget {
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

    pub(crate) fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        cx: &Context<'_>,
        in_flight: Option<InFlightStatus<'_>>,
        hints: impl FnOnce() -> Vec<KeyHint>,
    ) {
        if let Some(error_message) = self.error_message.as_ref() {
            Self::render_error(area, buf, cx, error_message);
        } else if let Some(status) = in_flight {
            Self::render_in_flight(area, buf, cx, &status);
        } else {
            Self::render_key_hints(area, buf, cx, hints());
        }
    }

    fn render_in_flight(
        area: Rect,
        buf: &mut Buffer,
        cx: &Context<'_>,
        status: &InFlightStatus<'_>,
    ) {
        let suffix = match status.other_count() {
            0 => String::new(),
            count => format!(" (+{count})"),
        };
        let label = match status.progress() {
            Some(RequestProgress::TimelineWindow { loaded, target }) => {
                format!("{} {loaded}/{target}", status.kind().label())
            }
            None => status.kind().label().into_owned(),
        };
        let suffix_width = u16::try_from(suffix.chars().count()).unwrap_or(u16::MAX);
        let [spinner_area, label_area, suffix_area] = Layout::horizontal([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(suffix_width),
        ])
        .areas(area);

        let mut throbber_state = ThrobberState::default();
        throbber_state.calc_step(status.throbber_step());
        StatefulWidget::render(
            Throbber::default()
                .throbber_set(throbber::BRAILLE_EIGHT_DOUBLE)
                .use_type(WhichUse::Spin),
            spinner_area,
            buf,
            &mut throbber_state,
        );
        Paragraph::new(label)
            .style(cx.theme.prompt.background)
            .render(label_area, buf);
        Paragraph::new(suffix)
            .style(cx.theme.prompt.background)
            .render(suffix_area, buf);
    }

    fn render_key_hints(area: Rect, buf: &mut Buffer, cx: &Context<'_>, hints: Vec<KeyHint>) {
        // Aliases share a label so that the available width also fits other actions.
        let mut by_label = indexmap::IndexMap::<String, Vec<String>>::new();
        for hint in hints {
            by_label
                .entry(hint.description)
                .or_default()
                .push(hint.keys);
        }
        let mut spans = Vec::new();
        let mut width = 0;
        for (description, keys) in by_label {
            let span = Span::styled(
                format!("{}:{description}  ", keys.join("/")),
                cx.theme.prompt.key_desc,
            );
            if width + span.width() <= usize::from(area.width) {
                width += span.width();
                spans.push(span);
            }
        }
        Paragraph::new(Line::from(spans))
            .alignment(Alignment::Center)
            .style(cx.theme.prompt.background)
            .render(area, buf);
    }

    fn render_error(area: Rect, buf: &mut Buffer, cx: &Context<'_>, error_message: &str) {
        Paragraph::new(Line::from(error_message))
            .alignment(Alignment::Left)
            .wrap(Wrap { trim: true })
            .style(cx.theme.error.message)
            .render(area, buf);
    }
}
