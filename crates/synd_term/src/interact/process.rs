use crate::interact::{Interact, OpenBrowser, OpenBrowserError, OpenEditor};

pub struct ProcessInteractor {}

impl ProcessInteractor {
    pub fn new() -> Self {
        Self {}
    }
}

impl OpenBrowser for ProcessInteractor {
    fn open_browser(&self, url: url::Url) -> Result<(), super::OpenBrowserError> {
        open::that(url.as_str()).map_err(OpenBrowserError::from)
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
