use synd_feed::types::Category;

use crate::{
    command::{Command, FilterCommand},
    ui::widgets::filter::FilterLane,
};

use super::CommandId;

/// Action produced after a key binding is resolved.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum KeymapAction {
    Command(CommandId),
    Prompt(PromptAction),
    Filter(FilterAction),
}

impl KeymapAction {
    pub(crate) fn build_command(&self) -> Command {
        match self {
            Self::Command(command) => command.build(),
            Self::Prompt(action) => Command::Filter(action.clone().into()),
            Self::Filter(action) => Command::Filter(action.clone().into()),
        }
    }
}

impl From<CommandId> for KeymapAction {
    fn from(command: CommandId) -> Self {
        Self::Command(command)
    }
}

/// Text-editing action for prompt-style keymap layers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PromptAction {
    InsertChar(char),
    DeleteBackward,
}

impl From<PromptAction> for FilterCommand {
    fn from(action: PromptAction) -> Self {
        match action {
            PromptAction::InsertChar(ch) => Self::PromptInsertChar(ch),
            PromptAction::DeleteBackward => Self::PromptDeleteBackward,
        }
    }
}

/// Filter action whose target data is only known at runtime.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FilterAction {
    ToggleCategory {
        lane: FilterLane,
        category: Category<'static>,
    },
    ActivateAllCategories {
        lane: FilterLane,
    },
    DeactivateAllCategories {
        lane: FilterLane,
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
            FilterAction::ToggleCategory { lane, category } => {
                Self::ToggleFilterCategory { lane, category }
            }
            FilterAction::ActivateAllCategories { lane } => {
                Self::ActivateAllFilterCategories { lane }
            }
            FilterAction::DeactivateAllCategories { lane } => {
                Self::DeactivateAllFilterCategories { lane }
            }
        }
    }
}
