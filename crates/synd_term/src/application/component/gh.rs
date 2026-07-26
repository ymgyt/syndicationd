use crate::{
    application::{Direction, Populate},
    client::gh::FetchNotificationsParams,
    command::GhNotificationFilterOption,
    operation::{Operation, Operations},
    types::gh::{Notification, NotificationDetails, ThreadId},
    ui::widgets::gh_notifications::{
        GhNotificationFilterOptions, GhNotificationsWidget, NotificationPageUpdate,
    },
};

/// GitHub notification state machine.
pub(crate) struct GhComponent {
    pub(crate) notifications: GhNotificationsWidget,
}

impl GhComponent {
    pub(super) fn new() -> Self {
        Self {
            notifications: GhNotificationsWidget::new(),
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
        Operation::FetchGhNotifications {
            populate: Populate::Replace,
            params: self.notifications.reload(),
        }
    }

    pub(in crate::application) fn apply_notifications(
        &mut self,
        populate: Populate,
        notifications: Vec<Notification>,
    ) -> NotificationDetails {
        self.notifications
            .update_notifications(populate, NotificationPageUpdate::from(notifications))
    }

    pub(in crate::application) fn fetch_notification_details(
        &self,
        details: NotificationDetails,
    ) -> impl Into<Operations> {
        details
            .into_iter()
            .map(Operation::from)
            .chain(self.fetch_next_notifications_if_needed())
            .collect::<Vec<_>>()
    }

    pub(in crate::application) fn move_notification(&mut self, direction: Direction) {
        self.notifications.move_selection(direction);
    }

    pub(in crate::application) fn move_notification_first(&mut self) {
        self.notifications.move_first();
    }

    pub(in crate::application) fn move_notification_last(&mut self) {
        self.notifications.move_last();
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

    pub(in crate::application) fn open_filter_popup(&mut self) {
        self.notifications.open_filter_popup();
    }

    pub(in crate::application) fn close_filter_popup(&mut self) -> Option<Operation> {
        self.notifications
            .close_filter_popup()
            .map(|params| Self::fetch_notifications(Populate::Replace, params))
    }

    pub(in crate::application) fn is_filter_popup_open(&self) -> bool {
        self.notifications.is_filter_popup_open()
    }

    pub(in crate::application) fn toggle_filter_option(
        &mut self,
        option: &GhNotificationFilterOption,
    ) {
        self.notifications.toggle_filter_option(option);
    }

    pub(in crate::application) fn fetch_next_notifications_if_needed(&self) -> Option<Operation> {
        self.notifications
            .fetch_next_if_needed()
            .map(|params| Self::fetch_notifications(Populate::Append, params))
    }

    fn fetch_notifications(populate: Populate, params: FetchNotificationsParams) -> Operation {
        Operation::FetchGhNotifications { populate, params }
    }
}
