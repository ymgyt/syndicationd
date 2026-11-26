use std::{
    io::{self, ErrorKind},
    path::PathBuf,
    process::Command,
};

use itertools::Itertools;
use tracing::instrument;
use url::Url;

use super::{ExternalTools, OpenBrowserError, OpenEditorError};

/// Uses the system browser and editor, and the configured text browser.
pub struct SystemTools {
    text_browser: PathBuf,
    text_browser_args: Vec<String>,
}

impl SystemTools {
    pub fn new(text_browser: PathBuf, text_browser_args: Vec<String>) -> Self {
        Self {
            text_browser,
            text_browser_args,
        }
    }
}

impl ExternalTools for SystemTools {
    fn open_browser(&self, url: Url) -> Result<(), OpenBrowserError> {
        open::that(url.as_str()).map_err(OpenBrowserError::from)
    }

    #[instrument(skip(self))]
    fn open_text_browser(&self, url: Url) -> Result<(), OpenBrowserError> {
        let status = Command::new(&self.text_browser)
            .args(&self.text_browser_args)
            .arg(url.as_str())
            .status()
            .map_err(|err| {
                if err.kind() == ErrorKind::NotFound {
                    OpenBrowserError::CommandNotFound {
                        command: self.text_browser.clone(),
                    }
                } else {
                    err.into()
                }
            })?;

        if status.success() {
            Ok(())
        } else {
            let arguments = self
                .text_browser_args
                .iter()
                .map(String::as_str)
                .chain([url.as_str()])
                .join(" ");
            Err(io::Error::other(format!("{} {arguments}", self.text_browser.display())).into())
        }
    }

    fn open_editor(&self, initial_content: &str) -> Result<String, OpenEditorError> {
        edit::edit(initial_content).map_err(OpenEditorError::from)
    }
}
