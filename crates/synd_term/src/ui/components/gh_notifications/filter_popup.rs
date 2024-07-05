use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Padding, Widget},
};

use crate::{
    client::github::{FetchNotificationInclude, FetchNotificationParticipating},
    types::github::{Notification, PullRequestState, Reason, RepoVisibility, SubjectContext},
    ui::{
        components::{
            filter::{FilterResult, Filterable},
            gh_notifications::{
                GhNotificationFilterOptions, GhNotificationFilterOptionsState,
                GhNotificationFilterUpdater,
            },
        },
        icon, Context,
    },
};

#[derive(Clone, Debug, Default)]
pub(super) struct OptionFilterer {
    options: GhNotificationFilterOptions,
}

impl OptionFilterer {
    pub(super) fn new(options: GhNotificationFilterOptions) -> Self {
        Self { options }
    }
}

impl Filterable<Notification> for OptionFilterer {
    fn filter(&self, n: &Notification) -> FilterResult {
        // unread and participating are handled in rest api
        if let Some(visibility) = self.options.visibility {
            if visibility != n.repository.visibility {
                return FilterResult::Discard;
            }
        }
        if !self.options.pull_request_conditions.is_empty() {
            match n.subject_context.as_ref() {
                Some(SubjectContext::PullRequest(pr)) => {
                    if !self.options.pull_request_conditions.contains(&pr.state) {
                        return FilterResult::Discard;
                    }
                }
                _ => return FilterResult::Discard,
            }
        }
        if !self.options.reasons.is_empty() {
            let mut ok = false;
            for reason in &self.options.reasons {
                if (reason == &Reason::Mention
                    && (n.reason == Reason::TeamMention || n.reason == Reason::Mention))
                    || reason == &n.reason
                {
                    ok = true;
                }
            }
            if !ok {
                return FilterResult::Discard;
            }
        }
        FilterResult::Use
    }
}

pub(super) struct FilterPopup {
    pub(super) is_active: bool,
    options: GhNotificationFilterOptions,
    pending_options: Option<GhNotificationFilterOptions>,
}

impl FilterPopup {
    pub(super) fn new(options: GhNotificationFilterOptions) -> Self {
        Self {
            is_active: false,
            options,
            pending_options: None,
        }
    }

    pub(super) fn applied_options(&self) -> &GhNotificationFilterOptions {
        &self.options
    }

    pub(super) fn commit(&mut self) -> GhNotificationFilterOptionsState {
        if let Some(options) = self.pending_options.take() {
            let org = std::mem::replace(&mut self.options, options);
            if org != self.options {
                return GhNotificationFilterOptionsState::Changed(self.options.clone());
            }
        }
        GhNotificationFilterOptionsState::Unchanged
    }

    pub(super) fn update_options(&mut self, new: &GhNotificationFilterUpdater) {
        let mut pending = self
            .pending_options
            .take()
            .unwrap_or_else(|| self.options.clone());

        if new.toggle_include {
            pending.include = match pending.include {
                FetchNotificationInclude::OnlyUnread => FetchNotificationInclude::All,
                FetchNotificationInclude::All => FetchNotificationInclude::OnlyUnread,
            };
        }
        if new.toggle_participating {
            pending.participating = match pending.participating {
                FetchNotificationParticipating::OnlyParticipating => {
                    FetchNotificationParticipating::All
                }
                FetchNotificationParticipating::All => {
                    FetchNotificationParticipating::OnlyParticipating
                }
            };
        }
        if new.toggle_visilibty_public {
            pending.visibility = match pending.visibility {
                Some(RepoVisibility::Public) => None,
                Some(RepoVisibility::Private) | None => Some(RepoVisibility::Public),
            };
        }
        if new.toggle_visilibty_private {
            pending.visibility = match pending.visibility {
                Some(RepoVisibility::Private) => None,
                Some(RepoVisibility::Public) | None => Some(RepoVisibility::Private),
            };
        }
        if let Some(pr_state) = new.toggle_pull_request_condition {
            pending.toggle_pull_request_condition(pr_state);
        }
        if let Some(reason) = new.toggle_reason.as_ref() {
            pending.toggle_reason(reason);
        }

        self.pending_options = Some(pending);
    }
}

impl FilterPopup {
    #[allow(clippy::too_many_lines)]
    pub(super) fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let area = {
            let block = Block::new()
                .title_top("Filter")
                .title_alignment(Alignment::Center)
                .title_style(Style::new().add_modifier(Modifier::BOLD))
                .padding(Padding {
                    left: 2,
                    right: 2,
                    top: 2,
                    bottom: 2,
                })
                .borders(Borders::ALL)
                .style(cx.theme.base);
            let inner_area = block.inner(area);
            block.render(area, buf);
            inner_area
        };

        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            // Constraint::Fill(1),
        ]);
        let [status_area, participating_area, visibility_area, pull_request_area, reason_area] =
            vertical.areas(area);
        let options = self.pending_options.as_ref().unwrap_or(&self.options);

        // Render status
        {
            let mut spans = vec![
                Span::from(concat!(icon!(unread), " Status")).bold(),
                Span::from("         "),
            ];
            let mut unread = Span::from("Unread(u)").underlined();
            if options.include == FetchNotificationInclude::All {
                unread = unread.dim().not_underlined();
            }
            spans.push(unread);

            Line::from(spans).render(status_area, buf);
        }

        // Render participating
        {
            let mut spans = vec![
                Span::from(concat!(icon!(chat), " Participating")).bold(),
                Span::from("  "),
            ];
            let mut participating = Span::from("Participating(pa)").underlined();
            if options.participating == FetchNotificationParticipating::All {
                participating = participating.dim().not_underlined();
            }
            spans.push(participating);
            Line::from(spans).render(participating_area, buf);
        }

        // Render repository visibility
        {
            let mut spans = vec![
                Span::from(concat!(icon!(repository), " Repository")).bold(),
                Span::from("     "),
            ];
            let mut public = Span::from("Public(pb)").dim();
            let mut private = Span::from("Private(pr)").dim();
            match options.visibility {
                Some(RepoVisibility::Public) => public = public.not_dim().underlined(),
                Some(RepoVisibility::Private) => private = private.not_dim().underlined(),
                None => {}
            };
            spans.extend([public, Span::from("  "), private]);
            Line::from(spans).render(visibility_area, buf);
        }

        // Render pull request conditions
        {
            let mut spans = vec![
                Span::from(concat!(icon!(pullrequest), " PullRequest")).bold(),
                Span::from("    "),
            ];
            let mut open = Span::from("Open(po)").dim();
            let mut merged = Span::from("Merged(pm)").dim();
            let mut closed = Span::from("Closed(pc)").dim();
            for cond in &options.pull_request_conditions {
                match cond {
                    PullRequestState::Open => {
                        open = open.not_dim().underlined();
                    }
                    PullRequestState::Merged => {
                        merged = merged.not_dim().underlined();
                    }
                    PullRequestState::Closed => {
                        closed = closed.not_dim().underlined();
                    }
                }
            }
            spans.extend([open, Span::from("  "), merged, Span::from("  "), closed]);
            Line::from(spans).render(pull_request_area, buf);
        }

        // Render reasons
        {
            let mut spans = vec![
                Span::from(concat!(icon!(chat), " Reason")).bold(),
                Span::from("         "),
            ];
            let mut mentioned = Span::from("Mentioned(me)").dim();
            let mut review = Span::from("ReviewRequested(rr)").dim();
            for reason in &options.reasons {
                match reason {
                    Reason::Mention | Reason::TeamMention => {
                        mentioned = mentioned.not_dim().underlined();
                    }
                    Reason::ReviewRequested => {
                        review = review.not_dim().underlined();
                    }
                    _ => {}
                }
            }
            spans.extend([mentioned, Span::from("  "), review]);
            Line::from(spans).render(reason_area, buf);
        }
    }
}
