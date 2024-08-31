use std::borrow::Cow;

use ratatui::{
    prelude::{Alignment, Buffer, Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    application::RequestId,
    ui::{
        icon,
        widgets::throbber::{
            throbber::{self, WhichUse},
            Throbber, ThrobberState,
        },
        Context,
    },
};

use super::tabs::Tab;

pub struct StatusLine {
    error_message: Option<String>,
}

impl StatusLine {
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

impl StatusLine {
    pub fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>, tab: Option<Tab>) {
        match self.error_message.as_ref() {
            Some(error_message) => Self::render_error(area, buf, cx, error_message),
            None => Self::render_prompt(area, buf, cx, tab),
        }
    }

    #[allow(clippy::cast_possible_truncation)]
    fn render_prompt(area: Rect, buf: &mut Buffer, cx: &Context<'_>, tab: Option<Tab>) {
        let pre_keys = &[
            ("Tab", "󰹳"),
            ("j/k", "󰹹"),
            ("gg", "󱞧"),
            ("ge", "󱞥"),
            ("c", icon!(category)),
            ("/", icon!(search)),
        ][..];
        let suf_keys = &[("r", "󰑓"), ("q", "")][..];
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
                .chain(suf_keys),
            Some(Tab::Entries) => pre_keys
                .iter()
                .chain(&[
                    ("h/l", icon!(requirement)),
                    ("Ent", icon!(open)),
                    ("Sp", icon!(browse)),
                ])
                .chain(suf_keys),
            Some(Tab::GitHub) => pre_keys
                .iter()
                .chain(&[
                    ("f", icon!(filter)),
                    ("Ent", icon!(open)),
                    ("d", icon!(check)),
                    ("u", ""),
                ])
                .chain(suf_keys),
            // Imply login
            None => [("j/k", "󰹹")][..]
                .iter()
                .chain(&[("Ent", "󰏌")])
                .chain(&[("q", "")][..]),
        };

        let spans = per_tab_keys
            .flat_map(|(key, desc)| {
                let desc = Span::styled(format!("{key}:{desc}  "), cx.theme.prompt.key_desc);
                [desc]
            })
            .collect::<Vec<_>>();

        let area = {
            if let Some(in_flight) = cx.in_flight.recent_in_flight() {
                let label = match in_flight {
                    RequestId::DeviceFlowDeviceAuthorize => {
                        Cow::Borrowed("Request device authorization")
                    }
                    RequestId::DeviceFlowPollAccessToken => Cow::Borrowed("Polling..."),
                    RequestId::FetchEntries => Cow::Borrowed("Fetch entries..."),
                    RequestId::FetchSubscription => Cow::Borrowed("Fetch subscription..."),
                    RequestId::FetchGithubNotifications { page } => {
                        Cow::Owned(format!("Fetch github notifications(page: {page})..."))
                    }
                    RequestId::FetchGithubIssue { id } => {
                        Cow::Owned(format!("Fetch github issue(#{id})..."))
                    }
                    RequestId::FetchGithubPullRequest { id } => {
                        Cow::Owned(format!("Fetch github pull request(#{id})..."))
                    }
                    RequestId::SubscribeFeed => Cow::Borrowed("Subscribe feed..."),
                    RequestId::UnsubscribeFeed => Cow::Borrowed("Unsubscribe feed..."),
                    RequestId::MarkGithubNotificationAsDone { id } => {
                        Cow::Owned(format!("Mark notification({id}) as done..."))
                    }
                    RequestId::UnsubscribeGithubThread => Cow::Borrowed("Unsubscribe thread..."),
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
