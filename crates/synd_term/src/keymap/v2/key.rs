use std::{fmt, str::FromStr};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{CommandId, KeymapError};

/// Normalized key used for keymap lookup.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct KeyStroke {
    code: KeyCode,
    modifiers: KeyModifiers,
}

impl KeyStroke {
    pub(crate) fn new(code: KeyCode, modifiers: KeyModifiers) -> Self {
        Self { code, modifiers }
    }

    pub(crate) fn from_char(ch: char) -> Self {
        Self::new(KeyCode::Char(ch), KeyModifiers::NONE)
    }

    pub(crate) fn as_char(self) -> Option<char> {
        if self
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        match self.code {
            KeyCode::Char(ch) => Some(ch),
            _ => None,
        }
    }
}

impl From<KeyEvent> for KeyStroke {
    fn from(event: KeyEvent) -> Self {
        Self::new(event.code, event.modifiers)
    }
}

impl From<crate::keymap::KeyNotation> for KeyStroke {
    fn from(notation: crate::keymap::KeyNotation) -> Self {
        let (code, modifiers) = notation.into_parts();
        Self::new(code, modifiers)
    }
}

impl FromStr for KeyStroke {
    type Err = KeymapError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let notation = value
            .parse::<crate::keymap::KeyNotation>()
            .map_err(|err| KeymapError::InvalidKeyNotation(value.to_owned(), err.to_string()))?;
        Ok(Self::from(notation))
    }
}

impl fmt::Display for KeyStroke {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::with_capacity(4);
        if self.modifiers.contains(KeyModifiers::CONTROL) {
            parts.push("C".to_owned());
        }
        if self.modifiers.contains(KeyModifiers::ALT) {
            parts.push("A".to_owned());
        }
        if self.modifiers.contains(KeyModifiers::SHIFT) {
            parts.push("S".to_owned());
        }

        let code = match self.code {
            KeyCode::Backspace => "backspace".to_owned(),
            KeyCode::Enter => "enter".to_owned(),
            KeyCode::Left => "left".to_owned(),
            KeyCode::Right => "right".to_owned(),
            KeyCode::Up => "up".to_owned(),
            KeyCode::Down => "down".to_owned(),
            KeyCode::Home => "home".to_owned(),
            KeyCode::End => "end".to_owned(),
            KeyCode::PageUp => "pageup".to_owned(),
            KeyCode::PageDown => "pagedown".to_owned(),
            KeyCode::Tab => "tab".to_owned(),
            KeyCode::BackTab => "backtab".to_owned(),
            KeyCode::Delete => "delete".to_owned(),
            KeyCode::Insert => "insert".to_owned(),
            KeyCode::F(n) => format!("f{n}"),
            KeyCode::Char(' ') => "space".to_owned(),
            KeyCode::Char(c) => c.to_string(),
            KeyCode::Esc => "esc".to_owned(),
            KeyCode::Null => "null".to_owned(),
            KeyCode::CapsLock => "capslock".to_owned(),
            KeyCode::ScrollLock => "scrolllock".to_owned(),
            KeyCode::NumLock => "numlock".to_owned(),
            KeyCode::PrintScreen => "printscreen".to_owned(),
            KeyCode::Pause => "pause".to_owned(),
            KeyCode::Menu => "menu".to_owned(),
            KeyCode::KeypadBegin => "keypadbegin".to_owned(),
            KeyCode::Media(_) | KeyCode::Modifier(_) => format!("{:?}", self.code),
        };
        parts.push(code);
        f.write_str(&parts.join("-"))
    }
}

/// Ordered key strokes that must be typed to trigger one binding.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct KeySequence {
    keys: Vec<KeyStroke>,
}

impl KeySequence {
    pub(crate) fn new(keys: Vec<KeyStroke>) -> Result<Self, KeymapError> {
        if keys.is_empty() {
            Err(KeymapError::EmptyKeySequence)
        } else {
            Ok(Self { keys })
        }
    }

    pub(super) fn parse<const N: usize>(keys: [&str; N]) -> Result<Self, KeymapError> {
        let keys = keys
            .into_iter()
            .map(str::parse)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(keys)
    }

    pub(super) fn as_slice(&self) -> &[KeyStroke] {
        &self.keys
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys = self
            .keys
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" ");
        f.write_str(&keys)
    }
}

/// Static binding from a key sequence to a command id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct KeyBinding {
    pub(crate) on: KeySequence,
    pub(crate) command: CommandId,
    pub(crate) desc: Option<String>,
}

impl KeyBinding {
    pub(super) fn new(on: KeySequence, command: CommandId, desc: Option<String>) -> Self {
        Self { on, command, desc }
    }
}
