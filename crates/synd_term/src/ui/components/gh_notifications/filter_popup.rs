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

    pub(super) fn options(&self) -> &GhNotificationFilterOptions {
        &self.options
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
    pending_options: Option<GhNotificationFilterOptions>,
}

impl FilterPopup {
    pub(super) fn new() -> Self {
        Self {
            is_active: false,
            pending_options: None,
        }
    }

    pub(super) fn commit(&mut self) -> GhNotificationFilterOptionsState {
        match self.pending_options.take() {
            Some(options) => GhNotificationFilterOptionsState::Changed(options),
            None => GhNotificationFilterOptionsState::Unchanged,
        }
    }

    pub(super) fn update_options(
        &mut self,
        new: &GhNotificationFilterUpdater,
        current: &GhNotificationFilterOptions,
    ) {
        let mut pending = self
            .pending_options
            .take()
            .unwrap_or_else(|| current.clone());

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
    pub(super) fn render(
        &self,
        area: Rect,
        buf: &mut Buffer,
        cx: &Context<'_>,
        current: &GhNotificationFilterOptions,
    ) {
        let area = {
            let block = Block::new()
                .title_top("Filter")
                .title_alignment(Alignment::Center)
                .title_style(Style::new().add_modifier(Modifier::BOLD))
                .padding(Padding {
                    left: 3,
                    right: 2,
                    top: 1,
                    bottom: 1,
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
        let options = self.pending_options.as_ref().unwrap_or(current);
        let keyword = cx.theme.entries.selected_entry;

        // Render status
        {
            let mut spans = vec![
                Span::from(concat!(icon!(unread), " Status")).bold(),
                Span::from("         "),
            ];
            let mut unread1 = Span::styled("Un", keyword).italic().bold();
            let mut unread2 = Span::from("read").bold();
            if options.include == FetchNotificationInclude::All {
                unread1 = unread1.dim();
                unread2 = unread2.dim();
            }
            spans.push(unread1);
            spans.push(unread2);

            Line::from(spans).render(status_area, buf);
        }

        // Render participating
        {
            let mut spans = vec![
                Span::from(concat!(icon!(chat), " Participating")).bold(),
                Span::from("  "),
            ];
            let mut par1 = Span::styled("Pa", keyword).italic().bold();
            let mut par2 = Span::from("rticipating").bold();
            if options.participating == FetchNotificationParticipating::All {
                par1 = par1.dim();
                par2 = par2.dim();
            }
            spans.extend([par1, par2]);
            Line::from(spans).render(participating_area, buf);
        }

        // Render repository visibility
        {
            let mut spans = vec![
                Span::from(concat!(icon!(repository), " Repository")).bold(),
                Span::from("     "),
            ];
            let mut pub1 = Span::styled("Pu", keyword).italic().bold();
            let mut pub2 = Span::from("blic").bold();
            let mut pri1 = Span::styled("Pr", keyword).italic().bold();
            let mut pri2 = Span::from("ivate").bold();
            match options.visibility {
                Some(RepoVisibility::Public) => {
                    pri1 = pri1.dim();
                    pri2 = pri2.dim();
                }
                Some(RepoVisibility::Private) => {
                    pub1 = pub1.dim();
                    pub2 = pub2.dim();
                }
                None => {
                    pri1 = pri1.dim();
                    pri2 = pri2.dim();
                    pub1 = pub1.dim();
                    pub2 = pub2.dim();
                }
            };
            spans.extend([pub1, pub2, Span::from("         "), pri1, pri2]);
            Line::from(spans).render(visibility_area, buf);
        }

        // Render pull request conditions
        {
            let mut spans = vec![
                Span::from(concat!(icon!(pullrequest), " PullRequest")).bold(),
                Span::from("    "),
            ];
            // dim then bold approach does not work :(
            let mut open1 = Span::styled("Op", keyword).italic().bold();
            let mut open2 = Span::from("en").bold();
            let mut merged1 = Span::styled("M", keyword).italic().bold();
            let mut merged2 = Span::from("e").bold();
            let mut merged3 = Span::styled("r", keyword).italic().bold();
            let mut merged4 = Span::from("ged").bold();
            let mut closed1 = Span::styled("Cl", keyword).italic().bold();
            let mut closed2 = Span::from("osed").bold();
            let mut disable_open = true;
            let mut disable_merged = true;
            let mut disable_closed = true;
            for cond in &options.pull_request_conditions {
                match cond {
                    PullRequestState::Open => {
                        disable_open = false;
                    }
                    PullRequestState::Merged => {
                        disable_merged = false;
                    }
                    PullRequestState::Closed => disable_closed = false,
                }
            }
            if disable_open {
                open1 = open1.dim();
                open2 = open2.dim();
            }
            if disable_merged {
                merged1 = merged1.dim();
                merged2 = merged2.dim();
                merged3 = merged3.dim();
                merged4 = merged4.dim();
            }
            if disable_closed {
                closed1 = closed1.dim();
                closed2 = closed2.dim();
            }
            spans.extend([
                open1,
                open2,
                Span::from("           "),
                merged1,
                merged2,
                merged3,
                merged4,
                Span::from("           "),
                closed1,
                closed2,
            ]);
            Line::from(spans).render(pull_request_area, buf);
        }

        // Render reasons
        {
            let mut spans = vec![
                Span::from(concat!(icon!(chat), " Reason")).bold(),
                Span::from("         "),
            ];
            let mut mentioned1 = Span::styled("Me", keyword).italic().bold();
            let mut mentioned2 = Span::from("ntioned").bold();
            let mut review1 = Span::styled("Re", keyword).italic().bold();
            let mut review2 = Span::from("viewRequested").bold();
            let mut disable_mentioned = true;
            let mut disable_review = true;
            for reason in &options.reasons {
                match reason {
                    Reason::Mention | Reason::TeamMention => {
                        disable_mentioned = false;
                    }
                    Reason::ReviewRequested => {
                        disable_review = false;
                    }
                    _ => {}
                }
            }
            if disable_mentioned {
                mentioned1 = mentioned1.dim();
                mentioned2 = mentioned2.dim();
            }
            if disable_review {
                review1 = review1.dim();
                review2 = review2.dim();
            }

            spans.extend([
                mentioned1,
                mentioned2,
                Span::from("      "),
                review1,
                review2,
            ]);
            Line::from(spans).render(reason_area, buf);
        }
    }
}
