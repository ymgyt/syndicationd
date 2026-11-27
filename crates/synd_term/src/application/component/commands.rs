use crate::application::overlay::OverlayId;
use crate::event::Event;
use crate::{
    application::{RequestKind, effects::Effects},
    command::GhFilterCommand,
    ui::widgets::gh_notifications::FilterPopup,
};
impl FilterPopup {
    pub(super) fn apply_command(command: GhFilterCommand, effects: &mut Effects) {
        match command {
            GhFilterCommand::Open => effects.open(OverlayId::GitHub),
            GhFilterCommand::Close => {
                effects.close();
                effects.emit(Event::GhFilterEditingFinished);
            }
        }
    }
}

impl Event {
    pub(super) fn request_failed(kind: &RequestKind, message: String) -> Self {
        tracing::error!(?kind, %message, "request failed");
        Self::ErrorReported(message)
    }
}
