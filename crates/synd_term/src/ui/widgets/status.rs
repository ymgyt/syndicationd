use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    application::{InFlightStatus, RequestProgress},
    ui::{
        Context, icon,
        widgets::throbber::{
            Throbber, ThrobberState,
            throbber::{self, WhichUse},
        },
    },
};

use super::tabs::Tab;

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

    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>, tab: Option<Tab>) {
        if let Some(error_message) = self.error_message.as_ref() {
            Self::render_error(area, buf, cx, error_message);
        } else if let Some(status) = cx.in_flight.status() {
            Self::render_in_flight(area, buf, cx, &status);
        } else {
            Self::render_key_hints(area, buf, cx, tab);
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

    #[allow(clippy::cast_possible_truncation)]
    fn render_key_hints(area: Rect, buf: &mut Buffer, cx: &Context<'_>, tab: Option<Tab>) {
        let pre_keys = &[
            ("Tab", "󰹳"),
            ("j/k", "󰹹"),
            ("gg", "󱞧"),
            ("ge", "󱞥"),
            ("c", icon!(category)),
            ("/", icon!(search)),
        ][..];
        let suffix_keys = &[("r", "󰑓"), ("q", "")][..];
        let per_tab_keys = match tab {
            Some(Tab::Feeds) => pre_keys
                .iter()
                .chain(&[
                    ("h/l", icon!(requirement)),
                    ("Ent", icon!(open)),
                    ("a", "󰑫"),
                    ("e", ""),
                    ("d", "󰼡"),
                ])
                .chain(suffix_keys),
            Some(Tab::Entries) => pre_keys
                .iter()
                .chain(&[
                    ("h/l", icon!(requirement)),
                    ("Ent", icon!(open)),
                    ("Sp", icon!(browse)),
                ])
                .chain(suffix_keys),
            Some(Tab::Gh) => pre_keys
                .iter()
                .chain(&[
                    ("f", icon!(filter)),
                    ("Ent", icon!(open)),
                    ("d", icon!(check)),
                    ("u", ""),
                ])
                .chain(suffix_keys),
            None => [("j/k", "󰹹")][..]
                .iter()
                .chain(&[("Ent", "󰏌")])
                .chain(&[("q", "")][..]),
        };
        let spans = per_tab_keys
            .map(|(key, desc)| Span::styled(format!("{key}:{desc}  "), cx.theme.prompt.key_desc))
            .collect::<Vec<_>>();

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
