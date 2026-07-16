#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TerminalFocus {
    Gained,
    Lost,
}

pub(super) struct State {
    pub(super) should_quit: bool,
    focus: TerminalFocus,
}

impl State {
    pub(super) fn new() -> Self {
        Self {
            should_quit: false,
            focus: TerminalFocus::Gained,
        }
    }

    pub(super) fn focus(&self) -> TerminalFocus {
        self.focus
    }

    pub(super) fn focus_gained(&mut self) {
        self.focus = TerminalFocus::Gained;
    }

    pub(super) fn focus_lost(&mut self) {
        self.focus = TerminalFocus::Lost;
    }
}
