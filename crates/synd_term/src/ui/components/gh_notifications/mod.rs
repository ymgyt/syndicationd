use std::{borrow::Cow, collections::HashMap, fmt::Debug, ops::ControlFlow};

use chrono_humanize::{Accuracy, HumanTime, Tense};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style, Styled, Stylize},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Cell, Padding, Paragraph, Row, Widget, Wrap},
};
use serde::{Deserialize, Serialize};

use crate::{
    application::{Direction, Populate},
    client::github::{
        FetchNotificationInclude, FetchNotificationParticipating, FetchNotificationsParams,
    },
    command::Command,
    config::{self, Categories},
    types::{
        TimeExt,
        github::{
            Comment, IssueContext, Notification, NotificationId, PullRequestContext,
            PullRequestState, Reason, RepoVisibility, SubjectContext, SubjectType,
        },
    },
    ui::{
        Context,
        components::{
            collections::FilterableVec,
            filter::{CategoryFilterer, ComposedFilterer, MatcherFilterer},
        },
        extension::RectExt,
        icon,
        widgets::{scrollbar::Scrollbar, table::Table},
    },
};

mod filter_popup;
use filter_popup::{FilterPopup, OptionFilterer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotificationStatus {
    MarkingAsDone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GhNotificationFilterOptionsState {
    Unchanged,
    Changed(GhNotificationFilterOptions),
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub(crate) struct GhNotificationFilterOptions {
    pub(crate) include: FetchNotificationInclude,
    pub(crate) participating: FetchNotificationParticipating,
    pub(crate) visibility: Option<RepoVisibility>,
    pub(crate) pull_request_conditions: Vec<PullRequestState>,
    pub(crate) reasons: Vec<Reason>,
}

impl Default for GhNotificationFilterOptions {
    fn default() -> Self {
        Self {
            include: FetchNotificationInclude::OnlyUnread,
            participating: FetchNotificationParticipating::All,
            visibility: None,
            pull_request_conditions: Vec::new(),
            reasons: Vec::new(),
        }
    }
}

impl GhNotificationFilterOptions {
    fn toggle_pull_request_condition(&mut self, pr_state: PullRequestState) {
        if let Some(idx) = self
            .pull_request_conditions
            .iter()
            .position(|cond| cond == &pr_state)
        {
            self.pull_request_conditions.swap_remove(idx);
        } else {
            self.pull_request_conditions.push(pr_state);
        }
    }

    fn toggle_reason(&mut self, reason: &Reason) {
        if let Some(idx) = self.reasons.iter().position(|r| r == reason) {
            self.reasons.swap_remove(idx);
        } else {
            self.reasons.push(reason.clone());
        }
    }
}

#[allow(clippy::struct_excessive_bools, clippy::struct_field_names)]
#[derive(Debug, Clone, Default)]
pub(crate) struct GhNotificationFilterUpdater {
    pub(crate) toggle_include: bool,
    pub(crate) toggle_participating: bool,
    pub(crate) toggle_visilibty_public: bool,
    pub(crate) toggle_visilibty_private: bool,
    pub(crate) toggle_pull_request_condition: Option<PullRequestState>,
    pub(crate) toggle_reason: Option<Reason>,
}

type CategoryAndMatcherFilterer = ComposedFilterer<CategoryFilterer, MatcherFilterer>;

#[allow(clippy::struct_field_names)]
pub(crate) struct GhNotifications {
    max_repository_name: usize,
    notifications:
        FilterableVec<Notification, ComposedFilterer<CategoryAndMatcherFilterer, OptionFilterer>>,

    #[allow(clippy::zero_sized_map_values)]
    status: HashMap<NotificationId, NotificationStatus>,
    limit: usize,
    next_page: Option<u8>,
    filter_popup: FilterPopup,
}

impl GhNotifications {
    pub(crate) fn new() -> Self {
        Self::with_filter_options(GhNotificationFilterOptions::default())
    }

    pub(crate) fn with_filter_options(filter_options: GhNotificationFilterOptions) -> Self {
        let filterer =
            CategoryAndMatcherFilterer::default().and_then(OptionFilterer::new(filter_options));

        Self {
            notifications: FilterableVec::from_filter(filterer),
            max_repository_name: 0,
            #[allow(clippy::zero_sized_map_values)]
            status: HashMap::new(),
            limit: config::github::NOTIFICATION_PER_PAGE as usize,
            next_page: Some(config::github::INITIAL_PAGE_NUM),
            filter_popup: FilterPopup::new(),
        }
    }

    pub(crate) fn filter_options(&self) -> &GhNotificationFilterOptions {
        self.notifications.filter().right().options()
    }

    pub(crate) fn update_filter_options(&mut self, updater: &GhNotificationFilterUpdater) {
        let current = self.filter_options().clone();
        self.filter_popup.update_options(updater, &current);
    }

    pub(crate) fn update_filterer(&mut self, filterer: CategoryAndMatcherFilterer) {
        self.notifications
            .with_filter(|composed| composed.update_left(filterer));
    }

    pub(crate) fn update_notifications(
        &mut self,
        populate: Populate,
        notifications: Vec<Notification>,
    ) -> Option<Command> {
        if notifications.is_empty() {
            self.next_page = None;
            return None;
        }

        match populate {
            Populate::Replace => self.next_page = Some(config::github::INITIAL_PAGE_NUM + 1),
            Populate::Append => self.next_page = self.next_page.map(|next| next.saturating_add(1)),
        }
        let contexts = notifications
            .iter()
            .filter_map(Notification::context)
            .collect();

        self.max_repository_name = self.max_repository_name.max(
            notifications
                .iter()
                .map(|n| n.repository.name.as_str().len())
                .max()
                .unwrap_or(0)
                .min(30),
        );

        self.notifications.update(populate, notifications);

        Some(Command::FetchGhNotificationDetails { contexts })
    }

    pub(crate) fn fetch_next_if_needed(&self) -> Option<Command> {
        match self.next_page {
            Some(page) if self.notifications.len() < self.limit => {
                tracing::debug!(
                    "Should fetch more. notifications: {} next_page {page}",
                    self.notifications.len(),
                );
                Some(Command::FetchGhNotifications {
                    populate: Populate::Append,
                    params: self.next_fetch_params(page),
                })
            }
            _ => {
                tracing::debug!(
                    "Nothing to fetch. notifications: {} next_page {:?}",
                    self.notifications.len(),
                    self.next_page
                );
                None
            }
        }
    }

    pub(crate) fn reload(&mut self) -> FetchNotificationsParams {
        self.next_page = Some(config::github::INITIAL_PAGE_NUM);
        self.next_fetch_params(config::github::INITIAL_PAGE_NUM)
    }

    fn next_fetch_params(&self, page: u8) -> FetchNotificationsParams {
        let options = self.filter_options();
        FetchNotificationsParams {
            page,
            include: options.include,
            participating: options.participating,
        }
    }

    pub(crate) fn update_issue(
        &mut self,
        notification_id: NotificationId,
        issue: IssueContext,
        config: &Categories,
    ) -> Option<&Notification> {
        let mut issue = Some(issue);
        self.notifications.with_mut(|n| {
            if n.id == notification_id {
                n.subject_context = Some(SubjectContext::Issue(issue.take().unwrap()));
                n.update_categories(config);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
    }

    pub(crate) fn update_pull_request(
        &mut self,
        notification_id: NotificationId,
        pull_request: PullRequestContext,
        config: &Categories,
    ) -> Option<&Notification> {
        let mut pull_request = Some(pull_request);
        self.notifications.with_mut(|n| {
            if n.id == notification_id {
                n.subject_context = Some(SubjectContext::PullRequest(pull_request.take().unwrap()));
                n.update_categories(config);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        })
    }

    pub(crate) fn marking_as_done(&mut self) -> Option<NotificationId> {
        let id = self.selected_notification()?.id;
        self.status.insert(id, NotificationStatus::MarkingAsDone);
        Some(id)
    }

    pub(crate) fn marking_as_done_all(&mut self) -> Vec<NotificationId> {
        let ids: Vec<NotificationId> = self.notifications.iter().map(|n| n.id).collect();
        for &id in &ids {
            self.status.insert(id, NotificationStatus::MarkingAsDone);
        }
        ids
    }

    pub(crate) fn marked_as_done(&mut self, id: NotificationId) {
        self.notifications.retain(|n| n.id != id);
    }

    pub(crate) fn move_selection(&mut self, direction: Direction) {
        self.notifications.move_selection(direction);
    }

    pub(crate) fn move_first(&mut self) {
        self.notifications.move_first();
    }

    pub(crate) fn move_last(&mut self) {
        self.notifications.move_last();
    }

    pub(crate) fn open_filter_popup(&mut self) {
        self.filter_popup.is_active = true;
    }

    #[must_use]
    pub(crate) fn close_filter_popup(&mut self) -> Option<Command> {
        self.filter_popup.is_active = false;
        match self.filter_popup.commit() {
            GhNotificationFilterOptionsState::Changed(new_options) => {
                (&new_options != self.filter_options()).then(|| {
                    self.apply_filter_options(new_options);
                    Command::FetchGhNotifications {
                        populate: Populate::Replace,
                        params: self.reload(),
                    }
                })
            }
            GhNotificationFilterOptionsState::Unchanged => None,
        }
    }

    fn apply_filter_options(&mut self, options: GhNotificationFilterOptions) {
        let filterer = OptionFilterer::new(options);
        self.notifications
            .with_filter(|composed| composed.update_right(filterer));
    }

    pub(crate) fn selected_notification(&self) -> Option<&Notification> {
        self.notifications.selected()
    }
}

impl GhNotifications {
    pub(crate) fn render(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let vertical = Layout::vertical([Constraint::Fill(2), Constraint::Fill(1)]);
        let [notifications_area, detail_area] = vertical.areas(area);

        self.render_notifications(notifications_area, buf, cx);
        self.render_detail(detail_area, buf, cx);

        if self.filter_popup.is_active {
            self.render_filter_popup(area, buf, cx);
        }
    }

    fn render_notifications(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let notifications_area = Block::new().padding(Padding::top(1)).inner(area);

        let (header, widths, rows) = self.notification_rows(cx);

        Table::builder()
            .header(header)
            .widths(widths)
            .rows(rows)
            .theme(&cx.theme.entries)
            .selected_idx(self.notifications.selected_index())
            .highlight_modifier(cx.table_highlight_modifier())
            .build()
            .render(notifications_area, buf);

        let header_rows = 2;
        #[allow(clippy::cast_possible_truncation)]
        let scrollbar_area = Rect {
            y: area.y + header_rows,
            height: area
                .height
                .saturating_sub(header_rows)
                .min(self.notifications.len() as u16),
            ..area
        };

        Scrollbar {
            content_length: self.notifications.len(),
            position: self.notifications.selected_index(),
        }
        .render(scrollbar_area, buf, cx);
    }

    fn notification_rows<'a>(
        &'a self,
        cx: &'a Context<'_>,
    ) -> (
        Row<'a>,
        impl IntoIterator<Item = Constraint>,
        impl IntoIterator<Item = Row<'a>>,
    ) {
        let (n, m) = {
            if self.notifications.is_empty() {
                (Cow::Borrowed("-"), Cow::Borrowed("-"))
            } else {
                (
                    Cow::Owned((self.notifications.selected_index() + 1).to_string()),
                    Cow::Owned(self.notifications.len().to_string()),
                )
            }
        };
        let header = Row::new([
            Cell::from("Updated"),
            Cell::from(format!("Title {n}/{m}")),
            Cell::from("Repository"),
            Cell::from("Reason"),
        ]);

        let constraints = [
            Constraint::Length(8),
            Constraint::Fill(1),
            Constraint::Max(self.max_repository_name.try_into().unwrap_or(30)),
            Constraint::Length(10),
        ];

        let row = |n: &'a Notification| {
            let updated_at = HumanTime::from(n.updated_at.signed_duration_since(cx.now))
                .to_text_en(Accuracy::Rough, Tense::Past);
            let updated_at = short_human_time(&updated_at);
            let subject = n.title();
            let subject_icon = n.subject_icon();
            let repo = n.repository.name.as_str();
            let reason = reason_label(&n.reason);

            let is_marking_as_done = self
                .status
                .get(&n.id)
                .is_some_and(|s| *s == NotificationStatus::MarkingAsDone);
            let modifier = if is_marking_as_done {
                Modifier::CROSSED_OUT | Modifier::DIM
            } else {
                Modifier::empty()
            };
            Row::new([
                Cell::from(Span::from(updated_at).add_modifier(modifier)),
                Cell::from(
                    Line::from(vec![subject_icon, Span::from(" "), Span::from(subject)])
                        .add_modifier(modifier),
                ),
                Cell::from(Span::from(repo).add_modifier(modifier)),
                Cell::from(Span::from(reason).add_modifier(modifier)),
            ])
        };
        (header, constraints, self.notifications.iter().map(row))
    }

    #[allow(clippy::too_many_lines)]
    fn render_detail(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let block = Block::new()
            .padding(Padding {
                left: 2,
                right: 2,
                top: 0,
                bottom: 0,
            })
            .borders(Borders::TOP)
            .border_type(BorderType::Plain);

        let inner = block.inner(area);
        Widget::render(block, area, buf);

        let Some(notification) = self.selected_notification() else {
            return;
        };

        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Fill(1),
        ]);

        let [subject_area, title_area, updated_at_area, content_area] = vertical.areas(inner);

        Line::from(vec![
            notification.subject_icon().bold(),
            Span::from(" Subject").bold(),
            Span::from("    "),
            Span::from(format!(
                "{} / {} ",
                notification.repository.owner.as_str(),
                notification.repository.name.as_str(),
            )),
            {
                let id = match notification.subject_type() {
                    Some(SubjectType::Issue) => {
                        format!(
                            "#{}",
                            notification
                                .issue_id()
                                .map(|id| id.to_string())
                                .unwrap_or_default()
                        )
                    }
                    Some(SubjectType::PullRequest) => {
                        format!(
                            "#{}",
                            notification
                                .pull_request_id()
                                .map(|id| id.to_string())
                                .unwrap_or_default()
                        )
                    }
                    Some(SubjectType::Ci) => "ci".to_owned(),
                    Some(SubjectType::Release) => "release".to_owned(),
                    Some(SubjectType::Discussion) => "discussion".to_owned(),
                    None => String::new(),
                };

                Span::from(id)
            },
        ])
        .render(subject_area, buf);

        Line::from(vec![
            Span::from(concat!(icon!(entry), " Title")).bold(),
            Span::from("      "),
            Span::from(notification.title()),
        ])
        .render(title_area, buf);

        Line::from(vec![
            Span::from(concat!(icon!(calendar), " UpdatedAt")).bold(),
            Span::from("  "),
            Span::from(notification.updated_at.local_ymd_hm()),
            {
                if let Some(last_read) = notification.last_read_at {
                    Span::from(format!(
                        " last read {}",
                        HumanTime::from(last_read.signed_duration_since(cx.now))
                            .to_text_en(Accuracy::Rough, Tense::Past)
                    ))
                    .dim()
                } else {
                    Span::from("")
                }
            },
        ])
        .render(updated_at_area, buf);

        let (label, padding) = match notification.subject_type() {
            Some(SubjectType::Issue) => ("Issue", "     "),
            Some(SubjectType::PullRequest) => ("PR", "        "),
            _ => ("Body", "      "),
        };
        let body = Line::from(vec![
            Span::from(format!("{} {label}", icon!(summary)))
                .bold()
                .underlined(),
            Span::from(padding),
            {
                if let Some(author) = notification.author() {
                    Span::from(format!(" @{author}")).dim()
                } else {
                    Span::from("")
                }
            },
        ]);
        let body_para = Paragraph::new(notification.body().unwrap_or_default())
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        let last_comment = notification.last_comment();

        // Render labels if exists
        let content_area = {
            let labels = notification.labels().map(|labels| {
                #[allow(unstable_name_collisions)]
                let labels = labels
                    .map(|label| {
                        let span = Span::from(&label.name);
                        if let Some(color) = label.color {
                            span.set_style(
                                Style::default().bg(color).fg(
                                    // Depending on the background color of the label
                                    // the foreground may become difficult to read
                                    cx.theme
                                        .contrast_fg_from_luminance(label.luminance.unwrap_or(0.5)),
                                ),
                            )
                        } else {
                            span
                        }
                    })
                    .intersperse(Span::from(" "));
                let mut line = vec![
                    Span::from(concat!(icon!(label), " Labels")).bold(),
                    Span::from("     "),
                ];
                line.extend(labels);
                Line::from(line)
            });
            match labels {
                None => content_area,
                Some(labels) => {
                    let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
                    let [labels_area, content_area] = vertical.areas(content_area);

                    labels.render(labels_area, buf);
                    content_area
                }
            }
        };

        if last_comment.is_none() {
            let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
            let [body_header_area, body_area] = vertical.areas(content_area);

            body.render(body_header_area, buf);
            body_para.render(body_area, buf);
        } else {
            let vertical = Layout::vertical([
                Constraint::Length(1),
                Constraint::Fill(1),
                Constraint::Length(1),
                Constraint::Fill(1),
            ]);
            let [
                body_header_area,
                body_area,
                comment_header_area,
                comment_area,
            ] = vertical.areas(content_area);

            body.render(body_header_area, buf);
            body_para.render(body_area, buf);

            #[expect(clippy::unnecessary_unwrap)]
            let Comment { author, body } = last_comment.unwrap();
            Line::from(vec![
                Span::from(concat!(icon!(comment), " Comment"))
                    .bold()
                    .underlined(),
                Span::from("   "),
                Span::from(format!(" @{author}")).dim(),
            ])
            .render(comment_header_area, buf);
            Paragraph::new(body)
                .wrap(Wrap { trim: true })
                .alignment(Alignment::Left)
                .render(comment_area, buf);
        }
    }

    fn render_filter_popup(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let area = {
            let vertical = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(9),
                Constraint::Fill(2),
            ]);
            let [_, area, _] = vertical.areas(area);

            let area = area.centered(70, 100);
            area.reset(buf);
            area
        };
        self.filter_popup
            .render(area, buf, cx, self.filter_options());
    }
}

fn reason_label(reason: &Reason) -> &str {
    match reason {
        Reason::Assign => "assigned",
        Reason::Author => "author",
        Reason::CiActivity => "ci",
        Reason::ManuallySubscribed => "manual",
        Reason::Mention => "mentioned",
        Reason::TeamMention => "team mentioned",
        Reason::ReviewRequested => "review",
        Reason::WatchingRepo => "subscribed",
        Reason::Other(other) => other,
    }
}

// 3 days ago => "_3d ago"
fn short_human_time(s: &str) -> String {
    if s == "now" {
        return s.to_owned();
    }
    let mut seg = s.splitn(3, ' ');

    let (Some(n), Some(u)) = (seg.next(), seg.next()) else {
        return s.to_owned();
    };
    let u = match u {
        "seconds" | "second" => "s",
        "minutes" | "minute" => "m",
        "hours" | "hour" => "h",
        "days" | "day" => "d",
        "weeks" | "week" => "w",
        "months" | "month" => "mo",
        "years" | "year" => "y",
        _ => "",
    };

    let n = match n {
        "an" | "a" => "1",
        n => n,
    };
    if u == "mo" {
        format!("{n}{u} ago")
    } else {
        format!("{n: >2}{u} ago")
    }
}
