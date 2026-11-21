use super::{Component, RenderContext};
use crate::{
    application::{Generation, Populate, RequestError, RequestKind, effects::Effects},
    client::gh::FetchNotificationsParams,
    command::{Command, GhCommand, GhNotificationFilterOption},
    operation::{Operation, Operations},
    types::gh::{Notification, NotificationDetails, ThreadId},
    ui::widgets::gh_notifications::{
        GhNotificationFilterOptions, GhNotificationsWidget, NotificationPageUpdate,
    },
};
use crate::{
    config::Categories,
    event::{Event, EventKind, GhEvent},
    keymap::Layer,
};
use ratatui::buffer::Buffer;
use std::rc::Rc;

/// GitHub notification state machine.
pub(crate) struct GhComponent {
    pub(in crate::application) enabled: bool,
    categories: Rc<Categories>,
    generation: Generation,
    page_pending: bool,
    matcher: crate::matcher::Matcher,
    pub(crate) notifications: GhNotificationsWidget,
}

impl GhComponent {
    pub(in crate::application) fn apply_command(&mut self, command: GhCommand) -> Effects {
        let operations = match command {
            GhCommand::OpenNotification => self.open_selected_notification().into(),
            GhCommand::OpenNotificationAndMarkAsDone => {
                self.open_selected_notification_and_mark_as_done().into()
            }
            GhCommand::ReloadNotifications => self.reload_notifications().into(),
            GhCommand::MarkNotificationAsDone => self.mark_selected_notification_as_done().into(),
            GhCommand::MarkAllNotificationsAsDone => self.mark_all_notifications_as_done().into(),
            GhCommand::UnsubscribeThread => self.unsubscribe_selected_thread().into(),
            GhCommand::ApplyNotificationFilter => self.close_filter_popup().into(),
            GhCommand::ToggleNotificationFilter(option) => {
                self.toggle_filter_option(&option);
                Operations::Nop
            }
        };
        Effects::operations(operations)
    }

    pub(super) fn new(enabled: bool, categories: Rc<Categories>) -> Self {
        Self {
            enabled,
            categories,
            matcher: crate::matcher::Matcher::new(),
            notifications: GhNotificationsWidget::new(),
            generation: Generation::default(),
            page_pending: false,
        }
    }

    pub(in crate::application) fn restore_filter_options(
        &mut self,
        options: GhNotificationFilterOptions,
    ) {
        self.notifications = GhNotificationsWidget::with_filter_options(options);
    }

    pub(in crate::application) fn filter_options_snapshot(&self) -> GhNotificationFilterOptions {
        self.notifications.filter_options().clone()
    }

    pub(in crate::application) fn reload_notifications(&mut self) -> Operation {
        let params = self.notifications.reload();
        self.start_list(params)
    }

    pub(in crate::application) fn apply_notifications(
        &mut self,
        populate: Populate,
        notifications: Vec<Notification>,
    ) -> NotificationDetails {
        self.page_pending = false;
        self.notifications
            .update_notifications(populate, NotificationPageUpdate::from(notifications))
    }

    pub(in crate::application) fn fetch_notification_details(
        &mut self,
        details: NotificationDetails,
    ) -> impl Into<Operations> {
        let generation = self.generation;
        details
            .into_iter()
            .map(|detail| Operation::FetchGhDetail { generation, detail })
            .chain(self.fetch_next_notifications_if_needed())
            .collect::<Vec<_>>()
    }

    pub(in crate::application) fn mark_selected_notification_as_done(&self) -> Option<Operation> {
        self.notifications
            .selected_notification_id()
            .map(|id| Operation::MarkGhNotificationAsDone { id })
    }

    pub(in crate::application) fn open_selected_notification_and_mark_as_done(
        &self,
    ) -> impl Into<Operations> {
        let open: Operations = self.open_selected_notification().into();
        let mark_as_done: Operations = self.mark_selected_notification_as_done().into();
        [open, mark_as_done]
    }

    pub(in crate::application) fn mark_all_notifications_as_done(&self) -> impl Into<Operations> {
        self.notifications
            .notification_ids()
            .into_iter()
            .map(|id| Operation::MarkGhNotificationAsDone { id })
            .collect::<Vec<_>>()
    }

    pub(in crate::application) fn unsubscribe_selected_thread(&self) -> impl Into<Operations> {
        let unsubscribe: Operations = self
            .selected_thread()
            .map(|id| Operation::UnsubscribeGhThread { id })
            .into();
        let mark_as_done: Operations = self.mark_selected_notification_as_done().into();
        [unsubscribe, mark_as_done]
    }

    pub(in crate::application) fn selected_thread(&self) -> Option<ThreadId> {
        self.notifications
            .selected_notification()
            .and_then(|notification| notification.thread_id)
    }

    pub(in crate::application) fn open_selected_notification(&self) -> Option<Operation> {
        self.notifications
            .selected_notification()
            .and_then(crate::types::gh::Notification::browser_url)
            .map(|url| Operation::OpenBrowser { url })
    }

    pub(in crate::application) fn close_filter_popup(&mut self) -> Option<Operation> {
        self.notifications
            .close_filter_popup()
            .map(|params| self.start_list(params))
    }

    pub(in crate::application) fn toggle_filter_option(
        &mut self,
        option: &GhNotificationFilterOption,
    ) {
        self.notifications.toggle_filter_option(option);
    }

    pub(super) fn fetch_next_notifications_if_needed(&mut self) -> Option<Operation> {
        if self.page_pending {
            return None;
        }
        let params = self.notifications.fetch_next_if_needed()?;
        Some(self.fetch_page(params))
    }

    fn start_list(&mut self, params: FetchNotificationsParams) -> Operation {
        self.generation.advance();
        self.fetch_page(params)
    }

    fn fetch_page(&mut self, params: FetchNotificationsParams) -> Operation {
        self.page_pending = true;
        let populate = if params.page == crate::config::gh::INITIAL_PAGE_NUM {
            Populate::Replace
        } else {
            Populate::Append
        };
        Operation::FetchGhNotifications {
            generation: self.generation,
            populate,
            params,
        }
    }
}

impl Component for GhComponent {
    fn render(&self, buf: &mut Buffer, cx: &RenderContext<'_>) {
        self.notifications.render(cx.frame.content, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::List, Layer::GhNotifications]
    }
    fn subscriptions(&self) -> &'static [EventKind] {
        &[
            EventKind::ApiAccessReady,
            EventKind::GhFilterChanged,
            EventKind::Gh,
            EventKind::RequestCompleted,
        ]
    }

    fn apply_event(&mut self, event: &Event) -> Effects {
        match event {
            Event::ApiAccessReady if self.enabled => {
                Effects::operations(self.reload_notifications())
            }
            Event::GhFilterChanged(criteria) => {
                self.notifications
                    .update_filterer(criteria.filterer(&mut self.matcher));
                Effects::operations(self.fetch_next_notifications_if_needed())
            }
            Event::RequestCompleted {
                kind,
                result: Err(RequestError::Gh(error)),
                ..
            } => {
                if matches!(kind, RequestKind::FetchGhNotifications { generation, .. }
                    | RequestKind::FetchGhIssue { generation, .. }
                    | RequestKind::FetchGhPullRequest { generation, .. } if *generation != self.generation)
                {
                    return Effects::NONE;
                }
                if matches!(kind, RequestKind::FetchGhNotifications { .. }) {
                    self.page_pending = false;
                }
                Command::show_request_error(kind, error.to_string()).into()
            }
            Event::Gh { event, .. } => self.apply_gh_event(event),
            _ => Effects::NONE,
        }
    }
}

impl GhComponent {
    fn apply_gh_event(&mut self, event: &GhEvent) -> Effects {
        if matches!(event, GhEvent::NotificationsFetched { generation, .. }
            | GhEvent::IssueFetched { generation, .. }
            | GhEvent::PullRequestFetched { generation, .. } if *generation != self.generation)
        {
            return Effects::NONE;
        }
        let notification = match event {
            GhEvent::NotificationsFetched {
                notifications,
                populate,
                ..
            } => {
                let details = self.apply_notifications(*populate, notifications.clone());
                let mut effects = Effects::operations(self.fetch_notification_details(details));
                if *populate == Populate::Replace {
                    effects.events.push(Event::GhCategoriesChanged {
                        populate: Populate::Replace,
                        categories: Vec::new(),
                    });
                }
                return effects;
            }
            GhEvent::IssueFetched {
                notification_id,
                issue,
                ..
            } => self
                .notifications
                .update_issue(*notification_id, issue.clone(), &self.categories),
            GhEvent::PullRequestFetched {
                notification_id,
                pull_request,
                ..
            } => self.notifications.update_pull_request(
                *notification_id,
                pull_request.clone(),
                &self.categories,
            ),
            GhEvent::NotificationMarkedAsDone { notification_id } => {
                self.notifications.marked_as_done(*notification_id);
                return Effects::NONE;
            }
        };
        notification.map_or(Effects::NONE, |n| {
            Event::GhCategoriesChanged {
                populate: Populate::Append,
                categories: n.categories().cloned().collect(),
            }
            .into()
        })
    }
}
