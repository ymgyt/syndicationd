use tracing::{info_span, instrument};

use crate::{command::Command, event::Event, operation::Operations};

use super::Compositor;
use crate::application::ComponentId;

/// Events from a state transition are applied before dispatching its operations.
#[must_use]
pub(in crate::application) struct Effects {
    events: Vec<Event>,
    operations: Operations,
    commands: Vec<Command>,
}

impl Effects {
    pub(in crate::application) const NONE: Self = Self {
        events: Vec::new(),
        operations: Operations::Nop,
        commands: Vec::new(),
    };

    pub(in crate::application) fn with_command(mut self, command: Command) -> Self {
        self.commands.push(command);
        self
    }

    pub(in crate::application) fn operations(operations: impl Into<Operations>) -> Self {
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

impl Compositor {
    #[instrument(skip_all)]
    pub(super) fn apply_command(&mut self, command: Command) -> Operations {
        let _guard = info_span!("apply_command", %command).entered();

        let effects = match command {
            Command::Open(id) => {
                self.overlay.open(self.components.shell.tabs.current(), id);
                Effects::NONE
            }
            Command::Close(id) => {
                self.overlay.close(self.components.shell.tabs.current(), id);
                Effects::NONE
            }
            Command::Shell(command) => self.components.shell.apply_command(command),
            Command::Feeds(command) => self
                .components
                .feeds
                .apply_command(command, self.feeds_per_pagination),
            Command::Filter(command) => self.components.shell.apply_filter_command(command),
            Command::Gh(command) => self.components.gh.apply_command(command),
            Command::List { to, command } => {
                match to {
                    ComponentId::Entries => self.components.feeds.entries.apply_list(command),
                    ComponentId::Feeds => self.components.feeds.subscription.apply_list(command),
                    ComponentId::GhNotifications => {
                        self.components.gh.notifications.apply_list(command);
                    }
                    _ => unreachable!("list command addressed to {to:?}"),
                }
                Effects::NONE
            }
            Command::Nop => Effects::NONE,
        };
        let operations = self.apply_effects(effects);
        self.sync_keymap();
        operations
    }

    fn apply_effects(&mut self, effects: Effects) -> Operations {
        if effects.events.is_empty() && effects.commands.is_empty() {
            return effects.operations;
        }
        let mut operations: Vec<_> = effects
            .events
            .into_iter()
            .map(|event| self.apply_event(event))
            .collect();
        operations.push(effects.operations);
        operations.extend(
            effects
                .commands
                .into_iter()
                .map(|command| self.apply_command(command)),
        );
        operations.into()
    }
}
