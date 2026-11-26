use std::cell::RefCell;

use url::Url;

use super::{ExternalTools, OpenBrowserError, OpenEditorError};

pub struct MockExternalTools {
    editor_buffer: RefCell<Vec<String>>,
    browser_urls: RefCell<Vec<String>>,
}

impl MockExternalTools {
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

impl ExternalTools for MockExternalTools {
    fn open_browser(&self, url: Url) -> Result<(), OpenBrowserError> {
        self.browser_urls.borrow_mut().push(url.to_string());
        Ok(())
    }

    fn open_text_browser(&self, url: Url) -> Result<(), OpenBrowserError> {
        self.open_browser(url)
    }

    fn open_editor(&self, _initial_content: &str) -> Result<String, OpenEditorError> {
        Ok(self.editor_buffer.borrow_mut().remove(0))
    }
}
