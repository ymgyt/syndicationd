use std::{
    io::{self, ErrorKind},
    path::PathBuf,
    process::{Command, Stdio},
};

use itertools::Itertools;
use url::Url;

use crate::interact::{Interact, OpenBrowserError, OpenEditor, OpenTextBrowser, OpenWebBrowser};

pub struct ProcessInteractor {
    text_browser: TextBrowserInteractor,
}

impl ProcessInteractor {
    pub fn new(text_browser: TextBrowserInteractor) -> Self {
        Self { text_browser }
    }
}

impl OpenWebBrowser for ProcessInteractor {
    fn open_browser(&self, url: url::Url) -> Result<(), super::OpenBrowserError> {
        open::that(url.as_str()).map_err(OpenBrowserError::from)
    }
}

impl OpenTextBrowser for ProcessInteractor {
    fn open_text_browser(&self, url: Url) -> Result<(), super::OpenBrowserError> {
        self.text_browser.open_text_browser(url)
    }
}

impl OpenEditor for ProcessInteractor {
    fn open_editor(&self, initial_content: &str) -> Result<String, super::OpenEditorError> {
        edit::edit(initial_content).map_err(|err| super::OpenEditorError {
            message: err.to_string(),
        })
    }
}

impl Interact for ProcessInteractor {}

pub struct TextBrowserInteractor {
    command: PathBuf,
    args: Vec<String>,
}

impl TextBrowserInteractor {
    pub fn new(command: PathBuf, args: Vec<String>) -> Self {
        Self { command, args }
    }
}

impl OpenTextBrowser for TextBrowserInteractor {
    #[tracing::instrument(skip(self))]
    fn open_text_browser(&self, url: Url) -> Result<(), OpenBrowserError> {
        let status = Command::new(self.command.as_os_str())
            .args(self.args.iter())
            .arg(url.as_str())
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .map_err(|err| {
                if err.kind() == ErrorKind::NotFound {
                    OpenBrowserError::CommandNotFound {
                        command: self.command.clone(),
                    }
                } else {
                    err.into()
                }
            })?
            .status;

        if status.success() {
            Ok(())
        } else {
            let full_command = if self.args.is_empty() {
                format!("{} {}", self.command.display(), url,)
            } else {
                format!(
                    "{} {} {}",
                    self.command.display(),
                    self.args.iter().join(" "),
                    url,
                )
            };
            Err(io::Error::new(io::ErrorKind::Other, full_command).into())
        }
    }
}
