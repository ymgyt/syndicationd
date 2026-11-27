use super::RenderContext;
use super::Screens;
use crate::{
    application::{Generation, Populate, RequestError, RequestKind, effects::Effects},
    client::gh::FetchNotificationsParams,
    command::{Command, GhCommand, GhNotificationFilterOption},
    operation::Operation,
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
use std::rc::Rc;
use tuiframework::Component;

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
    pub(in crate::application) fn apply_command(
        &mut self,
        command: GhCommand,
        effects: &mut Effects,
    ) {
        match command {
            GhCommand::OpenNotification => effects.operations(self.open_selected_notification()),
            GhCommand::OpenNotificationAndMarkAsDone => {
                self.open_selected_notification_and_mark_as_done(effects);
            }
            GhCommand::ReloadNotifications => effects.operation(self.reload_notifications()),
            GhCommand::MarkNotificationAsDone => {
                effects.operations(self.mark_selected_notification_as_done());
            }
            GhCommand::MarkAllNotificationsAsDone => {
                effects.operations(self.mark_all_notifications_as_done());
            }
            GhCommand::UnsubscribeThread => self.unsubscribe_selected_thread(effects),
            GhCommand::ToggleNotificationFilter(option) => {
                self.toggle_filter_option(&option);
            }
        }
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
    ) -> Vec<Operation> {
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

    fn open_selected_notification_and_mark_as_done(&self, effects: &mut Effects) {
        effects.operations(
            self.open_selected_notification()
                .into_iter()
                .chain(self.mark_selected_notification_as_done()),
        );
    }

    fn mark_all_notifications_as_done(&self) -> impl Iterator<Item = Operation> {
        self.notifications
            .notification_ids()
            .into_iter()
            .map(|id| Operation::MarkGhNotificationAsDone { id })
    }

    fn unsubscribe_selected_thread(&self, effects: &mut Effects) {
        let unsubscribe = self
            .selected_thread()
            .map(|id| Operation::UnsubscribeGhThread { id });
        effects.operations(
            unsubscribe
                .into_iter()
                .chain(self.mark_selected_notification_as_done()),
        );
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

impl Component<Screens> for GhComponent {
    fn apply_command(&mut self, command: Command, effects: &mut Effects) {
        match command {
            Command::Gh(action) => self.apply_command(action, effects),
            Command::List(action) => {
                self.notifications.apply_list(action);
            }
            _ => {}
        }
    }

    fn render(&self, frame: &mut ratatui::Frame<'_>, cx: &RenderContext<'_>) {
        let buf = frame.buffer_mut();
        self.notifications.render(cx.frame.content, buf, &cx.ui);
    }

    fn key_layers(&self) -> &[Layer] {
        &[Layer::List, Layer::GhNotifications]
    }
    fn subscribes(&self, event: &Event) -> bool {
        [
            EventKind::ApiAccessReady,
            EventKind::GhFilterEditingFinished,
            EventKind::GhFilterChanged,
            EventKind::Gh,
            EventKind::RequestCompleted,
        ]
        .contains(&EventKind::from(event))
    }

    fn apply_event(&mut self, event: &Event, effects: &mut Effects) {
        match event {
            Event::ApiAccessReady if self.enabled => effects.operation(self.reload_notifications()),
            Event::GhFilterEditingFinished => effects.operations(self.close_filter_popup()),
            Event::GhFilterChanged(criteria) => {
                self.notifications
                    .update_filterer(criteria.filterer(&mut self.matcher));
                effects.operations(self.fetch_next_notifications_if_needed());
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
                    return;
                }
                if matches!(kind, RequestKind::FetchGhNotifications { .. }) {
                    self.page_pending = false;
                }
                effects.emit(Event::request_failed(kind, error.to_string()));
            }
            Event::Gh { event, .. } => self.apply_gh_event(event, effects),
            _ => {}
        }
    }
}

impl GhComponent {
    fn apply_gh_event(&mut self, event: &GhEvent, effects: &mut Effects) {
        if matches!(event, GhEvent::NotificationsFetched { generation, .. }
            | GhEvent::IssueFetched { generation, .. }
            | GhEvent::PullRequestFetched { generation, .. } if *generation != self.generation)
        {
            return;
        }
        let notification = match event {
            GhEvent::NotificationsFetched {
                notifications,
                populate,
                ..
            } => {
                let details = self.apply_notifications(*populate, notifications.clone());
                effects.operations(self.fetch_notification_details(details));
                if *populate == Populate::Replace {
                    effects.emit(Event::GhCategoriesChanged {
                        populate: Populate::Replace,
                        categories: Vec::new(),
                    });
                }
                return;
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
                return;
            }
        };
        if let Some(n) = notification {
            effects.emit(Event::GhCategoriesChanged {
                populate: Populate::Append,
                categories: n.categories().cloned().collect(),
            });
        }
    }
}
