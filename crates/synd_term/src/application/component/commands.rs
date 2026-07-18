use crate::{
    command::{FeedsCommand, GitHubCommand, ShellCommand},
    keymap,
    operation::Operation,
    ui::widgets::filter::FilterLane,
};
use synd_feed::types::Category;

use super::{AppComponent, FeedsComponent};

impl AppComponent {
    pub(in crate::application) fn apply_shell_command(
        &mut self,
        command: ShellCommand,
        feeds_first: i64,
    ) -> Vec<Operation> {
        match command {
            ShellCommand::Quit => {
                self.shell.quit();
                Vec::new()
            }
            ShellCommand::Authenticate => self.shell.authenticate().into_iter().collect(),
            ShellCommand::MoveAuthenticationProvider(direction) => {
                self.shell.move_authentication_provider(direction);
                Vec::new()
            }
            ShellCommand::MoveTabSelection(direction) => {
                let tab = self.shell.move_tab_selection(direction);
                match tab {
                    crate::ui::widgets::tabs::Tab::Feeds if !self.feeds.has_subscription() => {
                        vec![Operation::FetchSubscription {
                            populate: crate::application::Populate::Replace,
                            after: None,
                            first: feeds_first,
                        }]
                    }
                    _ => Vec::new(),
                }
            }
            ShellCommand::RotateTheme => {
                self.shell.rotate_theme();
                Vec::new()
            }
        }
    }

    pub(in crate::application) fn apply_feeds_command(
        &mut self,
        command: FeedsCommand,
        feeds_first: i64,
        entries_first: i64,
    ) -> Vec<Operation> {
        match command {
            FeedsCommand::MoveSubscribedFeed(direction) => {
                self.feeds.move_subscription(direction);
                Vec::new()
            }
            FeedsCommand::MoveSubscribedFeedFirst => {
                self.feeds.move_subscription_first();
                Vec::new()
            }
            FeedsCommand::MoveSubscribedFeedLast => {
                self.feeds.move_subscription_last();
                Vec::new()
            }
            FeedsCommand::PromptFeedSubscription => vec![Operation::OpenFeedSubscriptionEditor],
            FeedsCommand::PromptFeedEdition => {
                self.feeds.edit_selected_feed().into_iter().collect()
            }
            FeedsCommand::PromptFeedUnsubscription => {
                self.feeds.open_unsubscribe_popup();
                Vec::new()
            }
            FeedsCommand::SelectFeedUnsubscriptionPopup => {
                let operation = self.feeds.selected_unsubscribe_operation();
                self.feeds.close_unsubscribe_popup();
                operation.into_iter().collect()
            }
            FeedsCommand::CancelFeedUnsubscriptionPopup => {
                self.feeds.close_unsubscribe_popup();
                Vec::new()
            }
            FeedsCommand::MoveFeedUnsubscriptionPopupSelection(direction) => {
                self.feeds.move_unsubscribe_popup_selection(direction);
                Vec::new()
            }
            FeedsCommand::ReloadSubscription => {
                vec![FeedsComponent::reload_subscription(feeds_first)]
            }
            FeedsCommand::OpenFeed => self.feeds.open_selected_feed().into_iter().collect(),
            FeedsCommand::ReloadEntries => {
                vec![FeedsComponent::reload_entries(entries_first)]
            }
            FeedsCommand::MoveEntry(direction) => {
                self.feeds.move_entry(direction);
                Vec::new()
            }
            FeedsCommand::MoveEntryFirst => {
                self.feeds.move_entry_first();
                Vec::new()
            }
            FeedsCommand::MoveEntryLast => {
                self.feeds.move_entry_last();
                Vec::new()
            }
            FeedsCommand::OpenEntry => self.feeds.open_selected_entry().into_iter().collect(),
            FeedsCommand::BrowseEntry => self.feeds.browse_selected_entry(),
        }
    }

    pub(in crate::application) fn apply_github_command(
        &mut self,
        command: GitHubCommand,
    ) -> Vec<Operation> {
        match command {
            GitHubCommand::MoveGhNotification(direction) => {
                self.github.move_notification(direction);
                Vec::new()
            }
            GitHubCommand::MoveGhNotificationFirst => {
                self.github.move_notification_first();
                Vec::new()
            }
            GitHubCommand::MoveGhNotificationLast => {
                self.github.move_notification_last();
                Vec::new()
            }
            GitHubCommand::OpenGhNotification { with_mark_as_done } => {
                self.github.open_selected_notification(with_mark_as_done)
            }
            GitHubCommand::ReloadGhNotifications => vec![self.github.reload_notifications()],
            GitHubCommand::MarkGhNotificationAsDone { all } => {
                self.github.mark_notification_as_done(all)
            }
            GitHubCommand::UnsubscribeGhThread => {
                let mut operations = self
                    .github
                    .selected_thread()
                    .map(|id| Operation::UnsubscribeGitHubThread { id })
                    .into_iter()
                    .collect::<Vec<_>>();
                operations.extend(self.github.mark_notification_as_done(false));
                operations
            }
            GitHubCommand::OpenGhNotificationFilterPopup => {
                self.github.open_filter_popup();
                Vec::new()
            }
            GitHubCommand::CloseGhNotificationFilterPopup => {
                let operation = self.github.close_filter_popup();
                operation.into_iter().collect()
            }
            GitHubCommand::UpdateGhnotificationFilterPopupOptions(updater) => {
                self.github.update_filter_popup_options(&updater);
                Vec::new()
            }
        }
    }

    pub(in crate::application) fn move_filter_requirement(
        &mut self,
        direction: crate::application::Direction,
    ) -> Vec<Operation> {
        let filterer = self.shell.move_filter_requirement(direction);
        self.apply_filterer(filterer).into_iter().collect()
    }

    pub(in crate::application) fn activate_category_filtering(&mut self) {
        self.shell.activate_category_filtering();
    }

    pub(in crate::application) fn category_filter_keymap(&self) -> Option<keymap::LayerKeymap> {
        self.shell.filter.category_filter_keymap()
    }

    pub(in crate::application) fn activate_search_filtering(&mut self) {
        self.shell.filter.activate_search_filtering();
    }

    pub(in crate::application) fn prompt_changed(&mut self) -> Vec<Operation> {
        if !self.shell.filter.is_search_active() {
            return Vec::new();
        }
        let filterer = self.shell.active_filterer();
        self.apply_filterer(filterer).into_iter().collect()
    }

    pub(in crate::application) fn insert_prompt_char(&mut self, ch: char) -> Vec<Operation> {
        self.shell.filter.insert_prompt_char(ch);
        self.prompt_changed()
    }

    pub(in crate::application) fn delete_prompt_backward(&mut self) -> Vec<Operation> {
        self.shell.filter.delete_prompt_backward();
        self.prompt_changed()
    }

    pub(in crate::application) fn deactivate_filtering(&mut self) {
        self.shell.filter.deactivate_filtering();
    }

    pub(in crate::application) fn toggle_filter_category(
        &mut self,
        category: &Category<'static>,
        lane: FilterLane,
    ) -> Vec<Operation> {
        let filterer = self.shell.toggle_filter_category(category, lane);
        self.apply_filterer(filterer).into_iter().collect()
    }

    pub(in crate::application) fn activate_all_filter_categories(
        &mut self,
        lane: FilterLane,
    ) -> Vec<Operation> {
        let filterer = self.shell.activate_all_filter_categories(lane);
        self.apply_filterer(filterer).into_iter().collect()
    }

    pub(in crate::application) fn deactivate_all_filter_categories(
        &mut self,
        lane: FilterLane,
    ) -> Vec<Operation> {
        let filterer = self.shell.deactivate_all_filter_categories(lane);
        self.apply_filterer(filterer).into_iter().collect()
    }
}
