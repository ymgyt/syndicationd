use crate::application::{ComponentId, component::Screens};
use synd_feed::types::Category;
use tuiframework::KeyAction;

use crate::command::{Command, FilterCommand, FilterTarget, PromptAction};

use super::CommandId;

/// Action produced after a key binding is resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum KeymapAction {
    Command(CommandId),
    Prompt(PromptAction),
    Filter(FilterAction),
}

impl KeyAction<Screens> for KeymapAction {
    fn command_for(self, owner: ComponentId) -> (ComponentId, Command) {
        let command = Command::from(self);
        (command.target(owner), command)
    }
}

impl From<KeymapAction> for Command {
    fn from(action: KeymapAction) -> Self {
        match action {
            KeymapAction::Command(command) => command.command(),
            KeymapAction::Prompt(action) => Self::Prompt(action),
            KeymapAction::Filter(action) => Self::Filter(action.into()),
        }
    }
}

impl From<CommandId> for KeymapAction {
    fn from(command: CommandId) -> Self {
        Self::Command(command)
    }
}

/// Filter action whose target data is only known at runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FilterAction {
    ToggleCategory {
        target: FilterTarget,
        category: Category<'static>,
    },
    ActivateAllCategories {
        target: FilterTarget,
    },
    DeactivateAllCategories {
        target: FilterTarget,
    },
}

impl From<FilterAction> for KeymapAction {
    fn from(action: FilterAction) -> Self {
        Self::Filter(action)
    }
}

impl From<FilterAction> for FilterCommand {
    fn from(action: FilterAction) -> Self {
        match action {
            FilterAction::ToggleCategory { target, category } => {
                Self::ToggleFilterCategory { target, category }
            }
            FilterAction::ActivateAllCategories { target } => {
                Self::ActivateAllFilterCategories { target }
            }
            FilterAction::DeactivateAllCategories { target } => {
                Self::DeactivateAllFilterCategories { target }
            }
        }
    }
}
