use synd_feed::types::Category;

use crate::{
    command::{FeedsCommand, FilterCommand, FilterTarget, GhCommand, ShellCommand},
    keymap,
    operation::{Operation, Operations},
    ui::widgets::{filter::Filterer, tabs::Tab},
};

use super::{Components, FeedsComponent};

/// Feed concern currently permitted to consume a direct command.
enum FeedsCommandState {
    Subscription,
    Timeline,
    UnsubscribePopup,
    Unavailable,
}

/// GitHub concern currently permitted to consume a direct command.
enum GhCommandState {
    Notifications,
    FilterPopup,
    Unavailable,
}

/// Runtime keymap derived from the current filter state.
pub(in crate::application) enum FilterKeymap {
    Normal,
    Category(keymap::LayerKeymap),
    Search(keymap::LayerKeymap),
}

impl FilterKeymap {
    pub(in crate::application) fn install(self, keymap: &mut keymap::Keymap) {
        keymap.clear_layer_keymap(keymap::Layer::CategoryFilter);
        keymap.clear_layer_keymap(keymap::Layer::SearchPrompt);
        match self {
            Self::Normal => {}
            Self::Category(layer) | Self::Search(layer) => keymap.set_layer_keymap(layer),
        }
    }
}

impl Components {
    pub(in crate::application) fn apply_shell_command(
        &mut self,
        command: ShellCommand,
        feeds_first: i64,
    ) -> Option<Operation> {
        if matches!(command, ShellCommand::MoveTabSelection(_)) && !self.shell.permits_main_ui() {
            return None;
        }

        match command {
            ShellCommand::Quit => {
                self.shell.quit();
                None
            }
            ShellCommand::Authenticate => self.shell.start_authentication(),
            ShellCommand::MoveAuthenticationProvider(direction) => {
                self.shell.move_authentication_provider(direction);
                None
            }
            ShellCommand::MoveTabSelection(direction) => {
                match self.shell.move_tab_selection(direction) {
                    Tab::Feeds if !self.feeds.has_subscription() => {
                        Some(FeedsComponent::reload_subscription(feeds_first))
                    }
                    _ => None,
                }
            }
            ShellCommand::RotateTheme => {
                self.shell.rotate_theme();
                None
            }
            ShellCommand::ForceRedraw => Some(Operation::ForceRedrawTerminal),
        }
    }

    pub(in crate::application) fn apply_feeds_command(
        &mut self,
        command: FeedsCommand,
        feeds_first: i64,
    ) -> Operations {
        match (self.feeds_command_state(), command) {
            (FeedsCommandState::Subscription, FeedsCommand::MoveSubscribedFeed(direction)) => {
                self.feeds.move_subscription(direction);
                Operations::Nop
            }
            (FeedsCommandState::Subscription, FeedsCommand::MoveSubscribedFeedFirst) => {
                self.feeds.move_subscription_first();
                Operations::Nop
            }
            (FeedsCommandState::Subscription, FeedsCommand::MoveSubscribedFeedLast) => {
                self.feeds.move_subscription_last();
                Operations::Nop
            }
            (FeedsCommandState::Subscription, FeedsCommand::PromptFeedSubscription) => {
                Operation::OpenFeedSubscriptionEditor.into()
            }
            (FeedsCommandState::Subscription, FeedsCommand::PromptFeedEdition) => {
                self.feeds.edit_selected_feed().into()
            }
            (FeedsCommandState::Subscription, FeedsCommand::PromptFeedUnsubscription) => {
                self.feeds.open_unsubscribe_popup();
                Operations::Nop
            }
            (
                FeedsCommandState::UnsubscribePopup,
                FeedsCommand::MoveFeedUnsubscriptionPopupSelection(direction),
            ) => {
                self.feeds.move_unsubscribe_popup_selection(direction);
                Operations::Nop
            }
            (FeedsCommandState::UnsubscribePopup, FeedsCommand::SelectFeedUnsubscriptionPopup) => {
                let operation = self.feeds.selected_unsubscribe_operation();
                self.feeds.close_unsubscribe_popup();
                operation.into()
            }
            (FeedsCommandState::UnsubscribePopup, FeedsCommand::CancelFeedUnsubscriptionPopup) => {
                self.feeds.close_unsubscribe_popup();
                Operations::Nop
            }
            (FeedsCommandState::Subscription, FeedsCommand::ReloadSubscription) => {
                FeedsComponent::reload_subscription(feeds_first).into()
            }
            (FeedsCommandState::Subscription, FeedsCommand::OpenFeed) => {
                self.feeds.open_selected_feed().into()
            }
            (FeedsCommandState::Timeline, FeedsCommand::RefreshTimeline) => {
                self.feeds.refresh_timeline().into()
            }
            (FeedsCommandState::Timeline, FeedsCommand::MoveEntry(direction)) => {
                self.feeds.move_entry(direction);
                Operations::Nop
            }
            (FeedsCommandState::Timeline, FeedsCommand::MoveEntryFirst) => {
                self.feeds.move_entry_first();
                Operations::Nop
            }
            (FeedsCommandState::Timeline, FeedsCommand::MoveEntryLast) => {
                self.feeds.move_entry_last();
                Operations::Nop
            }
            (FeedsCommandState::Timeline, FeedsCommand::OpenEntry) => {
                self.feeds.open_selected_entry().into()
            }
            (FeedsCommandState::Timeline, FeedsCommand::BrowseEntry) => {
                self.feeds.browse_selected_entry()
            }
            _ => Operations::Nop,
        }
    }

    pub(in crate::application) fn apply_gh_command(&mut self, command: GhCommand) -> Operations {
        match (self.gh_command_state(), command) {
            (GhCommandState::Notifications, GhCommand::MoveNotification(direction)) => {
                self.gh.move_notification(direction);
                Operations::Nop
            }
            (GhCommandState::Notifications, GhCommand::MoveNotificationFirst) => {
                self.gh.move_notification_first();
                Operations::Nop
            }
            (GhCommandState::Notifications, GhCommand::MoveNotificationLast) => {
                self.gh.move_notification_last();
                Operations::Nop
            }
            (GhCommandState::Notifications, GhCommand::OpenNotification) => {
                self.gh.open_selected_notification().into()
            }
            (GhCommandState::Notifications, GhCommand::OpenNotificationAndMarkAsDone) => {
                self.gh.open_selected_notification_and_mark_as_done().into()
            }
            (GhCommandState::Notifications, GhCommand::ReloadNotifications) => {
                self.gh.reload_notifications().into()
            }
            (GhCommandState::Notifications, GhCommand::MarkNotificationAsDone) => {
                self.gh.mark_selected_notification_as_done().into()
            }
            (GhCommandState::Notifications, GhCommand::MarkAllNotificationsAsDone) => {
                self.gh.mark_all_notifications_as_done().into()
            }
            (GhCommandState::Notifications, GhCommand::UnsubscribeThread) => {
                self.gh.unsubscribe_selected_thread().into()
            }
            (GhCommandState::Notifications, GhCommand::OpenNotificationFilter) => {
                self.gh.open_filter_popup();
                Operations::Nop
            }
            (GhCommandState::FilterPopup, GhCommand::CloseNotificationFilter) => {
                self.gh.close_filter_popup().into()
            }
            (GhCommandState::FilterPopup, GhCommand::ToggleNotificationFilter(option)) => {
                self.gh.toggle_filter_option(&option);
                Operations::Nop
            }
            _ => Operations::Nop,
        }
    }

    pub(in crate::application) fn apply_filter_command(
        &mut self,
        command: FilterCommand,
    ) -> Option<Operation> {
        if self.gh.is_filter_popup_open() {
            return None;
        }

        match command {
            FilterCommand::MoveFilterRequirement(direction)
                if self.shell.current_filter_target() == FilterTarget::Feeds =>
            {
                self.move_filter_requirement(direction)
            }
            FilterCommand::ActivateCategoryFiltering => {
                self.activate_category_filtering();
                None
            }
            FilterCommand::ActivateSearchFiltering => {
                self.activate_search_filtering();
                None
            }
            FilterCommand::PromptInsertChar(ch) if self.shell.filter.is_search_active() => {
                self.insert_prompt_char(ch)
            }
            FilterCommand::PromptDeleteBackward if self.shell.filter.is_search_active() => {
                self.delete_prompt_backward()
            }
            FilterCommand::DeactivateFiltering if self.shell.filter.is_filtering_active() => {
                self.deactivate_filtering();
                None
            }
            FilterCommand::ToggleFilterCategory { category, target }
                if self.shell.filter.category_filter_target() == Some(target) =>
            {
                self.toggle_filter_category(&category, target)
            }
            FilterCommand::ActivateAllFilterCategories { target }
                if self.shell.filter.category_filter_target() == Some(target) =>
            {
                self.activate_all_filter_categories(target)
            }
            FilterCommand::DeactivateAllFilterCategories { target }
                if self.shell.filter.category_filter_target() == Some(target) =>
            {
                self.deactivate_all_filter_categories(target)
            }
            FilterCommand::MoveFilterRequirement(_)
            | FilterCommand::PromptInsertChar(_)
            | FilterCommand::PromptDeleteBackward
            | FilterCommand::DeactivateFiltering
            | FilterCommand::ToggleFilterCategory { .. }
            | FilterCommand::ActivateAllFilterCategories { .. }
            | FilterCommand::DeactivateAllFilterCategories { .. } => None,
        }
    }

    fn feeds_command_state(&self) -> FeedsCommandState {
        if self.gh.is_filter_popup_open() {
            FeedsCommandState::Unavailable
        } else if self.feeds.is_unsubscribe_popup_open() {
            FeedsCommandState::UnsubscribePopup
        } else {
            match self.shell.tabs.current() {
                Tab::Feeds => FeedsCommandState::Subscription,
                Tab::Entries => FeedsCommandState::Timeline,
                Tab::Gh => FeedsCommandState::Unavailable,
            }
        }
    }

    fn gh_command_state(&self) -> GhCommandState {
        if self.gh.is_filter_popup_open() {
            GhCommandState::FilterPopup
        } else if self.shell.tabs.current() == Tab::Gh {
            GhCommandState::Notifications
        } else {
            GhCommandState::Unavailable
        }
    }

    pub(in crate::application) fn move_filter_requirement(
        &mut self,
        direction: crate::application::Direction,
    ) -> Option<Operation> {
        let filterer = self.shell.move_filter_requirement(direction);
        self.apply_filterer(filterer)
    }

    pub(in crate::application) fn activate_category_filtering(&mut self) {
        let target = self.shell.current_filter_target();
        self.shell.filter.activate_category_filtering(target);
    }

    pub(in crate::application) fn filter_keymap(&self) -> FilterKeymap {
        if let Some(category) = self.shell.filter.category_filter_keymap() {
            FilterKeymap::Category(category)
        } else if self.shell.filter.is_search_active() {
            FilterKeymap::Search(keymap::LayerKeymap::search_prompt())
        } else {
            FilterKeymap::Normal
        }
    }

    pub(in crate::application) fn activate_search_filtering(&mut self) {
        self.shell.filter.activate_search_filtering();
    }

    pub(in crate::application) fn insert_prompt_char(&mut self, ch: char) -> Option<Operation> {
        self.shell.filter.insert_prompt_char(ch);
        let filterer = self.shell.active_filterer();
        self.apply_filterer(filterer)
    }

    pub(in crate::application) fn delete_prompt_backward(&mut self) -> Option<Operation> {
        self.shell.filter.delete_prompt_backward();
        let filterer = self.shell.active_filterer();
        self.apply_filterer(filterer)
    }

    pub(in crate::application) fn deactivate_filtering(&mut self) {
        self.shell.filter.deactivate_filtering();
    }

    pub(in crate::application) fn toggle_filter_category(
        &mut self,
        category: &Category<'static>,
        target: FilterTarget,
    ) -> Option<Operation> {
        let filterer = self.shell.filter.toggle_category_state(category, target);
        self.apply_filterer(filterer)
    }

    pub(in crate::application) fn activate_all_filter_categories(
        &mut self,
        target: FilterTarget,
    ) -> Option<Operation> {
        let filterer = self.shell.filter.activate_all_categories_state(target);
        self.apply_filterer(filterer)
    }

    pub(in crate::application) fn deactivate_all_filter_categories(
        &mut self,
        target: FilterTarget,
    ) -> Option<Operation> {
        let filterer = self.shell.filter.deactivate_all_categories_state(target);
        self.apply_filterer(filterer)
    }

    #[must_use]
    fn apply_filterer(&mut self, filterer: Filterer) -> Option<Operation> {
        match filterer {
            Filterer::Feed(filterer) => {
                self.feeds.entries.update_filterer(filterer.clone());
                self.feeds.subscription.update_filterer(filterer);
                None
            }
            Filterer::GhNotification(filterer) => {
                self.gh.notifications.update_filterer(filterer);
                self.gh.fetch_next_notifications_if_needed()
            }
        }
    }
}
