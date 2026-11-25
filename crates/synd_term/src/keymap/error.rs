use thiserror::Error;

use super::{CommandId, Layer};

/// Error raised while parsing, validating, or compiling keymaps.
#[derive(Debug, Error)]
pub enum KeymapError {
    #[error("unknown keymap layer: {0}")]
    UnknownLayer(String),
    #[error("unknown command: {0}")]
    UnknownCommand(String),
    #[error("command `{command}` is not allowed in `{layer}` keymap layer")]
    CommandNotAllowed { layer: Layer, command: CommandId },
    #[error(transparent)]
    Bindings(#[from] tuiframework::keymap::KeymapError),
}
