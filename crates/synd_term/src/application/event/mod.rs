use crate::command::Command;

mod key_handlers;
pub use key_handlers::{KeyHandler, KeyHandlers};

#[expect(clippy::large_enum_variant)]
pub(crate) enum KeyEventResult {
    Consumed {
        command: Option<Command>,
        should_render: bool,
    },
    Ignored,
}

impl KeyEventResult {
    pub(super) fn is_consumed(&self) -> bool {
        matches!(self, KeyEventResult::Consumed { .. })
    }

    pub(crate) fn consumed(command: Command) -> Self {
        KeyEventResult::Consumed {
            command: Some(command),
            should_render: false,
        }
    }

    pub(crate) fn should_render(self, should_render: bool) -> Self {
        match self {
            KeyEventResult::Consumed { command, .. } => KeyEventResult::Consumed {
                command,
                should_render,
            },
            KeyEventResult::Ignored => KeyEventResult::Ignored,
        }
    }
}
