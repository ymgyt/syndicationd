use tracing::{info_span, instrument};

use crate::{
    command::{Command, FeedsCommand, FilterCommand, GitHubCommand, ShellCommand},
    keymap,
};

use super::Application;

impl Application {
    #[instrument(skip_all)]
    pub(super) fn apply_command(&mut self, command: Command) {
        let _guard = info_span!("apply_command", %command).entered();

        match command {
            Command::Nop => {}
            Command::Shell(command) => self.apply_shell_command(command),
            Command::Feeds(command) => self.apply_feeds_command(command),
            Command::Filter(command) => self.apply_filter_command(command),
            Command::GitHub(command) => self.apply_github_command(command),
        }
    }

    fn apply_shell_command(&mut self, command: ShellCommand) {
        let operations = self
            .components
            .apply_shell_command(command, self.config.feeds_per_pagination);
        self.perform_operations(operations);
    }

    fn apply_feeds_command(&mut self, command: FeedsCommand) {
        let operations = self.components.apply_feeds_command(
            command,
            self.config.feeds_per_pagination,
            self.next_entries_first(0),
        );
        self.perform_operations(operations);
    }

    fn apply_filter_command(&mut self, command: FilterCommand) {
        match command {
            FilterCommand::MoveFilterRequirement(direction) => {
                let operations = self.components.move_filter_requirement(direction);
                self.perform_operations(operations);
            }
            FilterCommand::ActivateCategoryFilterling => {
                self.components.activate_category_filtering();
                if let Some(layer_keymap) = self.components.category_filter_keymap() {
                    self.keymap.set_layer_keymap(layer_keymap);
                }
            }
            FilterCommand::ActivateSearchFiltering => {
                self.components.activate_search_filtering();
                self.keymap
                    .set_layer_keymap(keymap::LayerKeymap::search_prompt());
            }
            FilterCommand::PromptInsertChar(ch) => {
                let operations = self.components.insert_prompt_char(ch);
                self.perform_operations(operations);
            }
            FilterCommand::PromptDeleteBackward => {
                let operations = self.components.delete_prompt_backward();
                self.perform_operations(operations);
            }
            FilterCommand::DeactivateFiltering => {
                self.components.deactivate_filtering();
                self.keymap
                    .clear_layer_keymap(keymap::Layer::CategoryFilter);
                self.keymap.clear_layer_keymap(keymap::Layer::SearchPrompt);
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
        let operations = self.components.apply_github_command(command);
        self.perform_operations(operations);
    }
}
