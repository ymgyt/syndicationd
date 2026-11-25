use super::ComponentId;
use crate::{command::Command, event::Event, operation::Operations};

#[must_use]
pub(super) struct Effects {
    pub(super) events: Vec<Event>,
    pub(super) operations: Operations,
    pub(super) commands: Vec<Command>,
}

impl Effects {
    pub(super) const NONE: Self = Self {
        events: Vec::new(),
        operations: Operations::Nop,
        commands: Vec::new(),
    };

    pub(super) fn with_command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    pub(super) fn operations(operations: impl Into<Operations>) -> Self {
        Self {
            operations: operations.into(),
            ..Self::NONE
        }
    }
}

impl From<Event> for Effects {
    fn from(event: Event) -> Self {
        Self {
            events: vec![event],
            ..Self::NONE
        }
    }
}

impl From<Command> for Effects {
    fn from(command: Command) -> Self {
        Self::NONE.with_command(command)
    }
}

impl Effects {
    pub(in crate::application) fn into_framework(
        self,
        sender: ComponentId,
    ) -> tuiframework::Effects<super::compositor::SyndUi> {
        let mut effects = tuiframework::Effects::NONE;
        for event in self.events {
            effects = effects.emit(event);
        }
        for command in self.commands {
            effects = effects.send(command.target(sender), command);
        }
        for operation in self.operations {
            effects = effects.operation(operation);
        }
        effects
    }
}
