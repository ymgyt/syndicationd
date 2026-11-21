use super::Components;
use crate::{
    application::{
        ComponentId,
        effects::{ComponentEffects, Effects},
    },
    command::{Command, GhCommand, GhFilterCommand},
    ui::widgets::gh_notifications::FilterPopup,
};

impl Components {
    pub(in crate::application) fn apply_command(
        &mut self,
        command: Command,
    ) -> Option<ComponentEffects> {
        let (sender, effects) = match command {
            Command::Shell(command) => (ComponentId::Shell, self.shell.apply_command(command)),
            Command::Feeds(command) => (ComponentId::Feeds, self.feeds.apply_command(command)),
            Command::Filter(command) => (
                ComponentId::Filter,
                self.shell.apply_filter_command(command),
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
