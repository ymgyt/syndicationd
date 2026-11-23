use super::Components;
use crate::{
    application::{
        ComponentId, RequestKind,
        effects::{ComponentEffects, Effects},
    },
    command::{Command, GhCommand, GhFilterCommand, ShellCommand},
    ui::widgets::gh_notifications::FilterPopup,
};

impl Components {
    pub(in crate::application) fn apply_command(
        &mut self,
        command: Command,
    ) -> Option<ComponentEffects> {
        let (sender, effects) = match command {
            Command::Shell(command) => (ComponentId::Shell, self.shell.apply_command(command)),
            Command::Login(command) => (ComponentId::Login, self.login.apply_command(command)),
            Command::MoveTab(direction) => (ComponentId::Tabs, self.tabs.move_tab(direction)),
            Command::Feeds(command) => (ComponentId::Feeds, self.feeds.apply_command(command)),
            Command::Reader(command) => (ComponentId::Reader, self.reader.apply_command(command)),
            Command::Filter(command) => (
                ComponentId::Filter,
                self.filter
                    .apply_command(command, self.tabs.current().filter_target()),
            ),
            Command::Gh(command) => (ComponentId::GhNotifications, self.gh.apply_command(command)),
            Command::UnsubscribePopup(command) => (
                ComponentId::UnsubscribePopup,
                self.unsubscribe.apply_command(command),
            ),
            Command::GhFilterPopup(command) => (
                ComponentId::GhFilterPopup,
                FilterPopup::apply_command(command),
            ),
            Command::List { to, command } => {
                match to {
                    ComponentId::Entries => self.feeds.entries.apply_list(command),
                    ComponentId::Feeds => self.feeds.subscription.apply_list(command),
                    ComponentId::GhNotifications => self.gh.notifications.apply_list(command),
                    _ => unreachable!("list command addressed to {to:?}"),
                }
                (to, Effects::NONE)
            }
            Command::Nop | Command::Open | Command::Close => return None,
        };
        Some(effects.with_sender(sender))
    }
}

impl FilterPopup {
    fn apply_command(command: GhFilterCommand) -> Effects {
        match command {
            GhFilterCommand::Open => Effects::NONE.with_command(Command::Open),
            GhFilterCommand::Close => Effects::NONE
                .with_command(Command::Close)
                .with_command(Command::Gh(GhCommand::ApplyNotificationFilter)),
        }
    }
}

impl Command {
    pub(super) fn show_request_error(kind: &RequestKind, message: String) -> Self {
        tracing::error!(?kind, %message, "request failed");
        Self::Shell(ShellCommand::ShowError(message))
    }
}
