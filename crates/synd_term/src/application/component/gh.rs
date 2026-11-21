use crate::{
    application::{ComponentId, Populate, RequestKind, compositor::Effects},
    client::gh::FetchNotificationsParams,
    command::{Command, GhCommand, GhNotificationFilterOption},
    operation::{Operation, Operations},
    types::gh::{Notification, NotificationDetails, ThreadId},
    ui::widgets::filter::CategoryAndMatcherFilterer,
    ui::widgets::gh_notifications::{
        GhNotificationFilterOptions, GhNotificationsWidget, NotificationPageUpdate,
    },
};

/// GitHub notification state machine.
pub(crate) struct GhComponent {
    generation: u64,
    page_pending: bool,
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
            GhCommand::OpenNotificationFilter => {
                return Effects::NONE.with_command(Command::Open(ComponentId::GhFilterPopup));
            }
            GhCommand::CloseNotificationFilter => {
                return Effects::operations(self.close_filter_popup())
                    .with_command(Command::Close(ComponentId::GhFilterPopup));
            }
            GhCommand::ToggleNotificationFilter(option) => {
                self.toggle_filter_option(&option);
                Operations::Nop
            }
        };
        Effects::operations(operations)
    }

    pub(super) fn new() -> Self {
        Self {
            notifications: GhNotificationsWidget::new(),
            generation: 0,
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

    pub(in crate::application) fn bootstrap(&mut self) -> Operation {
        self.reload_notifications()
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

    pub(in crate::application) fn apply_filter(
        &mut self,
        filter: CategoryAndMatcherFilterer,
    ) -> Option<Operation> {
        self.notifications.update_filterer(filter);
        self.fetch_next_notifications_if_needed()
    }

    fn fetch_next_notifications_if_needed(&mut self) -> Option<Operation> {
        if self.page_pending {
            return None;
        }
        let params = self.notifications.fetch_next_if_needed()?;
        Some(self.fetch_page(params))
    }

    fn start_list(&mut self, params: FetchNotificationsParams) -> Operation {
        self.generation += 1;
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

    pub(in crate::application) fn is_stale(&self, kind: &RequestKind) -> bool {
        match kind {
            RequestKind::FetchGhNotifications { generation, .. }
            | RequestKind::FetchGhIssue { generation, .. }
            | RequestKind::FetchGhPullRequest { generation, .. } => *generation != self.generation,
            _ => false,
        }
    }

    pub(in crate::application) fn page_failed(&mut self) {
        self.page_pending = false;
    }
}
