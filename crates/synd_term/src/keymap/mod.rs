mod action;
mod command;
mod compiled;
mod config;
mod default;
mod error;
mod layer;
mod runtime;

pub(crate) use action::{FilterAction, KeymapAction, PromptAction};
pub use command::CommandId;
pub(crate) use command::CommandRegistry;
pub use compiled::CompiledKeymaps;
pub use config::KeymapConfig;
pub use error::KeymapError;
pub use layer::Layer;
pub(crate) use runtime::{LayerKeymap, search_prompt};
pub use tuiframework::keymap::KeySequence;
pub(crate) use tuiframework::keymap::KeyStroke;
pub(crate) type Keymap =
    tuiframework::keymap::Keymap<Layer, KeymapAction, crate::application::ComponentId>;
pub(crate) type LayerStack =
    tuiframework::keymap::LayerStack<Layer, crate::application::ComponentId>;
#[cfg(test)]
mod tests;
