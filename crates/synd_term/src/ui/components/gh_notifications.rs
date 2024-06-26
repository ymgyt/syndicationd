use std::{borrow::Cow, collections::HashMap};

use chrono_humanize::{Accuracy, HumanTime, Tense};
use itertools::Itertools;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style, Styled, Stylize},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Padding, Paragraph, Row, StatefulWidget, Table,
        TableState, Widget, Wrap,
    },
};

use crate::{
    application::{Direction, Populate},
    command::Command,
    config::{self, Categories},
    types::{
        github::{
            Comment, IssueContext, Notification, NotificationId, PullRequestContext,
            SubjectContext, SubjectType,
        },
        TimeExt,
    },
    ui::{
        self,
        components::{collections::FilterableVec, filter::GhNotificationFilterer},
        icon,
        widgets::scrollbar::Scrollbar,
        Context,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotificationStatus {
    MarkingAsDone,
}

#[allow(clippy::struct_field_names)]
pub(crate) struct GhNotifications {
    max_repository_name: usize,
    notifications: FilterableVec<Notification, GhNotificationFilterer>,

    #[allow(clippy::zero_sized_map_values)]
    status: HashMap<NotificationId, NotificationStatus>,
    limit: usize,
    next_page: Option<u8>,
}

impl GhNotifications {
    pub(crate) fn new() -> Self {
        Self {
            notifications: FilterableVec::new(),
            max_repository_name: 0,
            #[allow(clippy::zero_sized_map_values)]
            status: HashMap::new(),
            limit: config::github::NOTIFICATION_PER_PAGE as usize,
            next_page: Some(config::github::INITIAL_PAGE_NUM),
        }
    }

    pub(crate) fn update_filterer(&mut self, filterer: GhNotificationFilterer) {
        self.notifications.update_filter(filterer);
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
                Some(Command::FetchGhNotifications {
                    populate: Populate::Append,
                    page,
                })
            }
            _ => None,
        }
    }

    pub(crate) fn update_issue(
        &mut self,
        notification_id: NotificationId,
        issue: IssueContext,
        config: &Categories,
    ) -> Option<&Notification> {
        for n in self.notifications.unfiltered_iter_mut() {
            if n.id == notification_id {
                n.subject_context = Some(SubjectContext::Issue(issue));
                n.update_categories(config);
                return Some(n);
            }
        }
        None
    }

    pub(crate) fn update_pull_request(
        &mut self,
        notification_id: NotificationId,
        pull_request: PullRequestContext,
        config: &Categories,
    ) -> Option<&Notification> {
        for n in self.notifications.unfiltered_iter_mut() {
            if n.id == notification_id {
                n.subject_context = Some(SubjectContext::PullRequest(pull_request));
                n.update_categories(config);
                return Some(n);
            }
        }
        None
    }

    pub(crate) fn marking_as_done(&mut self) -> Option<NotificationId> {
        let id = self.selected_notification()?.id;
        self.status.insert(id, NotificationStatus::MarkingAsDone);
        Some(id)
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
    }

    fn render_notifications(&self, area: Rect, buf: &mut Buffer, cx: &Context<'_>) {
        let notifications_area = Block::new().padding(Padding::top(1)).inner(area);

        let mut notifications_state = TableState::new()
            .with_offset(0)
            .with_selected(self.notifications.selected_index());
        let (header, widths, rows) = self.notification_rows(cx);
        let notifications = Table::new(rows, widths)
            .header(header.style(cx.theme.entries.header))
            .column_spacing(2)
            .highlight_symbol(ui::TABLE_HIGHLIGHT_SYMBOL)
            .highlight_style(cx.theme.entries.selected_entry)
            .highlight_spacing(ratatui::widgets::HighlightSpacing::WhenSelected);

        StatefulWidget::render(
            notifications,
            notifications_area,
            buf,
            &mut notifications_state,
        );

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
            let reason = reason_label(n.reason.as_str());

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
            if labels.is_none() {
                content_area
            } else {
                let vertical = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]);
                let [labels_area, content_area] = vertical.areas(content_area);

                labels.unwrap().render(labels_area, buf);
                content_area
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
            let [body_header_area, body_area, comment_header_area, comment_area] =
                vertical.areas(content_area);

            body.render(body_header_area, buf);
            body_para.render(body_area, buf);

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
}

// https://docs.github.com/en/rest/activity/notifications?apiVersion=2022-11-28
fn reason_label(reason: &str) -> &str {
    match reason {
        "approval_requested" => "approval req",
        // Assigned to the issue
        "assign" => "assigned",
        // You created the thread
        "author" => "author",
        // You commented on the thread
        "comment" => "comment",
        // A GitHub Actions workflow run that you triggered was completed
        "ci_activity" => "ci",
        // You accepted an invitation to contriute to the repository
        "invitation" => "invitation",
        // You subscribed to the thread(via an issue or pull request)
        "manual" => "manual",
        // Organization members have requested to enable a feature such as Draft Pull Requests or Copilot
        "member_feature_requested" => "feature req",
        // You wre specifically @mentioned in the content
        "mention" => "mentioned",
        // You, or a team you're a member of, were requested to review a pull request
        "review_requested" => "review",
        // GitHub discovered a security vulnerability in your repo
        "security_alert" => "security alert",
        // You wre credited for contributing to a security advisory
        "security_advisory_credit" => "security advisory credit",
        // You changed the thread state (for example, closing an issue or merging a PR)
        "state_change" => "state change",
        // You're watching the repository
        "subscribed" => "subscribed",
        // You were on a team that was mentioned
        "team_mention" => "team mentioned",
        etc => etc,
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
