use tracing::{info_span, instrument};

use crate::{command::Command, operation::Operations};

use super::Application;

impl Application {
    #[instrument(skip_all)]
    pub(super) fn apply_command(&mut self, command: Command) -> Operations {
        let _guard = info_span!("apply_command", %command).entered();

        match command {
            Command::Shell(command) => self
                .components
                .apply_shell_command(command, self.config.feeds_per_pagination)
                .into(),
            Command::Feeds(command) if self.components.shell.permits_main_ui() => self
                .components
                .apply_feeds_command(command, self.config.feeds_per_pagination),
            Command::Filter(command) if self.components.shell.permits_main_ui() => {
                self.components.apply_filter_command(command).into()
            }
            Command::Gh(command) if self.components.shell.permits_main_ui() => {
                self.components.apply_gh_command(command)
            }
            Command::Nop | Command::Feeds(_) | Command::Filter(_) | Command::Gh(_) => {
                Operations::Nop
            }
        }
    }
}
