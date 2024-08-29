use std::io;

#[cfg(feature = "integration")]
pub mod mock;
mod process;
pub use process::{ProcessInteractor, TextBrowserInteractor};

use thiserror::Error;
use url::Url;

pub trait Interact: OpenWebBrowser + OpenTextBrowser + OpenEditor {}

#[derive(Debug, Error)]
pub enum OpenBrowserError {
    #[error("failed to open browser: {0}")]
    Io(#[from] io::Error),
}

pub trait OpenWebBrowser {
    fn open_browser(&self, url: Url) -> Result<(), OpenBrowserError>;
}

pub trait OpenTextBrowser {
    fn open_text_browser(&self, url: Url) -> Result<(), OpenBrowserError>;
}

#[derive(Debug, Error)]
#[error("failed to open editor: {message}")]
pub struct OpenEditorError {
    message: String,
}

pub trait OpenEditor {
    fn open_editor(&self, initial_content: &str) -> Result<String, OpenEditorError>;
}
