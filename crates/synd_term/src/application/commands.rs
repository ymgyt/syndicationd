use crate::{
    command::{Command, FeedsCommand, FilterCommand, GitHubCommand, ShellCommand},
    keymap::KeymapId,
    ui::widgets::tabs::Tab,
};

use super::Application;

impl Application {
    #[tracing::instrument(skip_all)]
    pub(super) fn apply_command(&mut self, command: Command) {
        let _guard = tracing::info_span!("apply_command", %command).entered();

        match command {
            Command::Shell(command) => self.apply_shell_command(command),
            Command::Feeds(command) => self.apply_feeds_command(command),
            Command::Filter(command) => self.apply_filter_command(command),
            Command::GitHub(command) => self.apply_github_command(command),
        }
    }

    fn apply_shell_command(&mut self, command: ShellCommand) {
        let move_tab = matches!(command, ShellCommand::MoveTabSelection(_));
        if move_tab {
            self.keymaps()
                .disable(KeymapId::Subscription)
                .disable(KeymapId::Entries)
                .disable(KeymapId::GhNotification);
        }

        let operations = self
            .components
            .apply_shell_command(command, self.config.feeds_per_pagination);

        if move_tab {
            match self.components.shell.tabs.current() {
                Tab::Feeds => {
                    self.keymaps().enable(KeymapId::Subscription);
                }
                Tab::Entries => {
                    self.keymaps().enable(KeymapId::Entries);
                }
                Tab::GitHub => {
                    self.keymaps().enable(KeymapId::GhNotification);
                }
            }
        }

        self.perform_operations(operations);
    }

    fn apply_feeds_command(&mut self, command: FeedsCommand) {
        match command {
            FeedsCommand::PromptFeedUnsubscription => {
                let operations = self.components.apply_feeds_command(
                    command,
                    self.config.feeds_per_pagination,
                    self.next_entries_first(0),
                );
                self.perform_operations(operations);
                if self.components.is_feed_unsubscription_popup_open() {
                    self.keymaps().enable(KeymapId::UnsubscribePopupSelection);
                }
            }
            FeedsCommand::SelectFeedUnsubscriptionPopup
            | FeedsCommand::CancelFeedUnsubscriptionPopup => {
                let operations = self.components.apply_feeds_command(
                    command,
                    self.config.feeds_per_pagination,
                    self.next_entries_first(0),
                );
                self.perform_operations(operations);
                self.keymaps().disable(KeymapId::UnsubscribePopupSelection);
            }
            command => {
                let operations = self.components.apply_feeds_command(
                    command,
                    self.config.feeds_per_pagination,
                    self.next_entries_first(0),
                );
                self.perform_operations(operations);
            }
        }
    }

    fn apply_filter_command(&mut self, command: FilterCommand) {
        match command {
            FilterCommand::MoveFilterRequirement(direction) => {
                let operations = self.components.move_filter_requirement(direction);
                self.perform_operations(operations);
            }
            FilterCommand::ActivateCategoryFilterling => {
                let keymap = self.components.activate_category_filtering();
                self.keymaps().update(KeymapId::CategoryFiltering, keymap);
            }
            FilterCommand::ActivateSearchFiltering => {
                let prompt = self.components.activate_search_filtering();
                self.key_handlers
                    .push(super::key_handlers::KeyHandler::Prompt(prompt));
            }
            FilterCommand::PromptChanged => {
                let operations = self.components.prompt_changed();
                self.perform_operations(operations);
            }
            FilterCommand::DeactivateFiltering => {
                self.components.deactivate_filtering();
                self.keymaps().disable(KeymapId::CategoryFiltering);
                self.key_handlers.remove_prompt();
            }
            FilterCommand::ToggleFilterCategory { category, lane } => {
                let operations = self.components.toggle_filter_category(&category, lane);
                self.perform_operations(operations);
            }
            FilterCommand::ActivateAllFilterCategories { lane } => {
                let operations = self.components.activate_all_filter_categories(lane);
                self.perform_operations(operations);
            }
            FilterCommand::DeactivateAllFilterCategories { lane } => {
                let operations = self.components.deactivate_all_filter_categories(lane);
                self.perform_operations(operations);
            }
        }
    }

    fn apply_github_command(&mut self, command: GitHubCommand) {
        match command {
            GitHubCommand::OpenGhNotificationFilterPopup => {
                let operations = self.components.apply_github_command(command);
                self.perform_operations(operations);
                self.keymaps().enable(KeymapId::GhNotificationFilterPopup);
                self.keymaps().disable(KeymapId::GhNotification);
                self.keymaps().disable(KeymapId::Filter);
                self.keymaps().disable(KeymapId::Entries);
                self.keymaps().disable(KeymapId::Subscription);
            }
            GitHubCommand::CloseGhNotificationFilterPopup => {
                let operations = self.components.apply_github_command(command);
                self.perform_operations(operations);
                self.keymaps().disable(KeymapId::GhNotificationFilterPopup);
                self.keymaps().enable(KeymapId::GhNotification);
                self.keymaps().enable(KeymapId::Filter);
                self.keymaps().enable(KeymapId::Entries);
                self.keymaps().enable(KeymapId::Subscription);
            }
            command => {
                let operations = self.components.apply_github_command(command);
                self.perform_operations(operations);
            }
        }
    }
}
