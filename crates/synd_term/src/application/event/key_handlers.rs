use std::{cell::RefCell, rc::Rc};

use crossterm::event::KeyEvent;

use crate::{application::event::KeyEventResult, keymap::Keymaps, ui::widgets::prompt::Prompt};

pub enum KeyHandler {
    Prompt(Rc<RefCell<Prompt>>),
    Keymaps(Keymaps),
}

impl KeyHandler {
    fn handle(&mut self, event: &KeyEvent) -> KeyEventResult {
        match self {
            KeyHandler::Prompt(prompt) => prompt.borrow_mut().handle_key_event(event),
            KeyHandler::Keymaps(keymaps) => keymaps.search(event),
        }
    }
}

pub struct KeyHandlers {
    handlers: Vec<KeyHandler>,
}

impl KeyHandlers {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn push(&mut self, handler: KeyHandler) {
        self.handlers.push(handler);
    }

    pub fn remove_prompt(&mut self) {
        self.handlers
            .retain(|h| !matches!(h, KeyHandler::Prompt(_)));
    }

    pub fn keymaps_mut(&mut self) -> Option<&mut Keymaps> {
        for handler in &mut self.handlers {
            match handler {
                KeyHandler::Keymaps(keymaps) => return Some(keymaps),
                KeyHandler::Prompt(_) => continue,
            }
        }
        None
    }

    pub fn handle(&mut self, event: KeyEvent) -> KeyEventResult {
        for handler in self.handlers.iter_mut().rev() {
            if let KeyEventResult::Consumed(r) = handler.handle(&event) {
                return KeyEventResult::Consumed(r);
            }
        }
        KeyEventResult::Ignored
    }
}
