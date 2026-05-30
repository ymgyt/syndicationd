use thiserror::Error;

use super::{CommandId, KeySequence, Layer};

#[derive(Debug, Error)]
pub enum KeymapError {
    #[error("unknown keymap layer: {0}")]
    UnknownLayer(String),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("command `{command}` is not allowed in `{layer}` keymap layer")]
    CommandNotAllowed { layer: Layer, command: CommandId },
    #[error("invalid key notation `{0}`: {1}")]
    InvalidKeyNotation(String, String),
    #[error("key sequence must contain at least one key")]
    EmptyKeySequence,
    #[error("duplicate binding in `{layer}` keymap layer: {keys}")]
    DuplicateBinding { layer: Layer, keys: KeySequence },
    #[error("ambiguous key binding: {keys}")]
    AmbiguousBinding { keys: KeySequence },
}
