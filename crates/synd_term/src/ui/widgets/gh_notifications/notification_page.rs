use crate::types::gh::{Notification, NotificationDetails};

/// State update derived once from one fetched GitHub notification page.
pub(crate) struct NotificationPageUpdate(NotificationPageUpdateState);

pub(super) enum NotificationPageUpdateState {
    /// An empty page proves that pagination has reached its end.
    Exhausted,
    /// A non-empty page and every result derived from that same ordered page.
    Fetched {
        notifications: Vec<Notification>,
        detail_targets: NotificationDetails,
        repository_name_width: usize,
    },
}

impl NotificationPageUpdate {
    pub(super) fn into_state(self) -> NotificationPageUpdateState {
        self.0
    }
}

impl From<Vec<Notification>> for NotificationPageUpdate {
    fn from(notifications: Vec<Notification>) -> Self {
        if notifications.is_empty() {
            return Self(NotificationPageUpdateState::Exhausted);
        }

        let mut detail_targets = Vec::new();
        let mut repository_name_width = 0;
        for notification in &notifications {
            repository_name_width =
                repository_name_width.max(notification.repository.name.len().min(30));
            if let Some(detail) = notification.detail() {
                detail_targets.push(detail);
            }
        }

        Self(NotificationPageUpdateState::Fetched {
            notifications,
            detail_targets: detail_targets.into_iter().collect(),
            repository_name_width,
        })
    }
}
