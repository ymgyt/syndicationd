use super::ComponentId;
use crate::{
    application::{RequestKind, effects::Effects},
    command::{Command, GhCommand, GhFilterCommand, ShellCommand},
    ui::widgets::gh_notifications::FilterPopup,
};
impl FilterPopup {
    pub(super) fn apply_command(command: GhFilterCommand) -> Effects {
        match command {
            GhFilterCommand::Open => Effects::NONE.send(ComponentId::GhFilterPopup, Command::Open),
            GhFilterCommand::Close => Effects::NONE
                .send(ComponentId::GhFilterPopup, Command::Close)
                .send(
                    ComponentId::GhNotifications,
                    Command::Gh(GhCommand::ApplyNotificationFilter),
                ),
        }
    }
}

impl Command {
    pub(super) fn show_request_error(kind: &RequestKind, message: String) -> Self {
        tracing::error!(?kind, %message, "request failed");
        Self::Shell(ShellCommand::ShowError(message))
    }
}
