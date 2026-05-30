use crate::{
    application::{Direction, Populate},
    client::github::FetchNotificationsParams,
    operation::Operation,
    types::github::ThreadId,
    ui::widgets::gh_notifications::{GhNotificationFilterUpdater, GitHubNotificationsWidget},
};

/// GitHub notification state machine.
pub(crate) struct GitHubComponent {
    pub(crate) notifications: GitHubNotificationsWidget,
}

impl GitHubComponent {
    pub(super) fn new() -> Self {
        Self {
            notifications: GitHubNotificationsWidget::new(),
        }
    }

    pub(in crate::application) fn reload_notifications(&mut self) -> Operation {
        Operation::FetchGitHubNotifications {
            populate: Populate::Replace,
            params: self.notifications.reload(),
        }
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

    pub(in crate::application) fn mark_notification_as_done(
        &mut self,
        all: bool,
    ) -> Vec<Operation> {
        let ids = if all {
            self.notifications.notification_ids()
        } else {
            let Some(id) = self.notifications.selected_notification_id() else {
                return Vec::new();
            };
            vec![id]
        };

        ids.into_iter()
            .map(|id| Operation::MarkGitHubNotificationAsDone { id })
            .collect()
    }

    pub(in crate::application) fn selected_thread(&self) -> Option<ThreadId> {
        self.notifications
            .selected_notification()
            .and_then(|notification| notification.thread_id)
    }

    pub(in crate::application) fn open_selected_notification(
        &mut self,
        with_mark_as_done: bool,
    ) -> Vec<Operation> {
        let mut operations = Vec::new();
        if let Some(url) = self
            .notifications
            .selected_notification()
            .and_then(crate::types::github::Notification::browser_url)
        {
            operations.push(Operation::OpenBrowser { url });
        }
        if with_mark_as_done {
            operations.extend(self.mark_notification_as_done(false));
        }
        operations
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

    pub(in crate::application) fn update_filter_popup_options(
        &mut self,
        updater: &GhNotificationFilterUpdater,
    ) {
        self.notifications.update_filter_options(updater);
    }

    pub(in crate::application) fn fetch_next_notifications_if_needed(&self) -> Option<Operation> {
        self.notifications
            .fetch_next_if_needed()
            .map(|params| Self::fetch_notifications(Populate::Append, params))
    }

    fn fetch_notifications(populate: Populate, params: FetchNotificationsParams) -> Operation {
        Operation::FetchGitHubNotifications { populate, params }
    }
}
