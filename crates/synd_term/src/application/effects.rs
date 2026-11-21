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

    pub(super) fn with_sender(self, sender: ComponentId) -> ComponentEffects {
        ComponentEffects {
            sender,
            effects: self,
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

pub(super) struct ComponentEffects {
    pub(super) sender: ComponentId,
    pub(super) effects: Effects,
}

impl From<Command> for Effects {
    fn from(command: Command) -> Self {
        Self::NONE.with_command(command)
    }
}
