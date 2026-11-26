use std::{io, path::PathBuf};

#[cfg(feature = "integration")]
pub mod mock;
mod system;
pub use system::SystemTools;

use thiserror::Error;
use url::Url;

/// Opens the user's browsers and editor on behalf of the TUI driver.
pub trait ExternalTools {
    fn open_browser(&self, url: Url) -> Result<(), OpenBrowserError>;
    fn open_text_browser(&self, url: Url) -> Result<(), OpenBrowserError>;
    fn open_editor(&self, initial_content: &str) -> Result<String, OpenEditorError>;
}

#[derive(Debug, Error)]
pub enum OpenBrowserError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("command `{command}` not found")]
    CommandNotFound { command: PathBuf },
}

#[derive(Debug, Error)]
#[error("failed to open editor: {0}")]
pub struct OpenEditorError(#[from] io::Error);
