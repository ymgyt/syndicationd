use crate::{
    command::{Command, FeedsCommand, FilterCommand, GitHubCommand, ShellCommand},
    keymap::v2,
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

        let operations = self
            .components
            .apply_shell_command(command, self.config.feeds_per_pagination);

        if move_tab {
            self.key_resolver_v2.clear_pending();
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
                    self.key_resolver_v2.clear_pending();
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
                self.key_resolver_v2.clear_pending();
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
                let _ = self.components.activate_category_filtering();
                if let Some(keymap) = self.components.category_filter_keymap_v2() {
                    self.keymaps_v2.set_layer_keymap(keymap);
                }
                self.key_resolver_v2.clear_pending();
            }
            FilterCommand::ActivateSearchFiltering => {
                let _ = self.components.activate_search_filtering();
                self.keymaps_v2
                    .set_layer_keymap(v2::LayerKeymap::search_prompt());
                self.key_resolver_v2.clear_pending();
            }
            FilterCommand::PromptInsertChar(ch) => {
                let operations = self.components.insert_prompt_char(ch);
                self.perform_operations(operations);
            }
            FilterCommand::PromptDeleteBackward => {
                let operations = self.components.delete_prompt_backward();
                self.perform_operations(operations);
            }
            FilterCommand::PromptChanged => {
                let operations = self.components.prompt_changed();
                self.perform_operations(operations);
            }
            FilterCommand::DeactivateFiltering => {
                self.components.deactivate_filtering();
                self.keymaps_v2
                    .clear_layer_keymap(v2::Layer::CategoryFilter);
                self.keymaps_v2.clear_layer_keymap(v2::Layer::SearchPrompt);
                self.key_resolver_v2.clear_pending();
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
            GitHubCommand::OpenGhNotificationFilterPopup
            | GitHubCommand::CloseGhNotificationFilterPopup => {
                let operations = self.components.apply_github_command(command);
                self.perform_operations(operations);
                self.key_resolver_v2.clear_pending();
            }
            command => {
                let operations = self.components.apply_github_command(command);
                self.perform_operations(operations);
            }
        }
    }
}
