use crate::command::Command;

use bitflags::bitflags;

bitflags! {
    pub(super) struct Should: u64 {
        const Render = 1 << 0;
        const Quit = 1 << 1;

    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalFocus {
    Gained,
    Lost,
}

pub(super) struct State {
    pub(super) flags: Should,
    focus: TerminalFocus,
}

impl State {
    pub(super) fn new() -> Self {
        Self {
            flags: Should::empty(),
            focus: TerminalFocus::Gained,
        }
    }

    pub(super) fn focus(&self) -> TerminalFocus {
        self.focus
    }

    pub(super) fn focus_gained(&mut self) -> Option<Command> {
        self.focus = TerminalFocus::Gained;
        None
    }

    pub(super) fn focus_lost(&mut self) -> Option<Command> {
        self.focus = TerminalFocus::Lost;
        None
    }
}
