use std::cell::RefCell;

use crate::interact::{Interact, OpenBrowser, OpenBrowserError, OpenEditor, OpenEditorError};

pub struct MockInteractor {
    editor_buffer: RefCell<Vec<String>>,
    browser_urls: RefCell<Vec<String>>,
}

impl MockInteractor {
    pub fn new() -> Self {
        Self {
            editor_buffer: RefCell::new(Vec::new()),
            browser_urls: RefCell::new(Vec::new()),
        }
    }

    #[must_use]
    pub fn with_buffer(mut self, editor_buffer: Vec<String>) -> Self {
        self.editor_buffer = RefCell::new(editor_buffer);
        self
    }
}

impl OpenBrowser for MockInteractor {
    fn open_browser(&self, url: url::Url) -> Result<(), OpenBrowserError> {
        self.browser_urls.borrow_mut().push(url.to_string());
        Ok(())
    }
}

impl OpenEditor for MockInteractor {
    fn open_editor(&self, _initial_content: &str) -> Result<String, OpenEditorError> {
        Ok(self.editor_buffer.borrow_mut().remove(0))
    }
}

impl Interact for MockInteractor {}
