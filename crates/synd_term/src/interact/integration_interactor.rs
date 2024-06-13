use std::{cell::RefCell, ffi::OsStr};

pub type Interactor = TestInteractor;

pub struct TestInteractor {
    editor_buffer: RefCell<Vec<String>>,
    browser_urls: RefCell<Vec<String>>,
}

impl TestInteractor {
    pub fn new() -> Self {
        Self {
            editor_buffer: RefCell::new(Vec::new()),
            browser_urls: RefCell::new(Vec::new()),
        }
    }

    pub fn with_buffer(mut self, editor_buffer: Vec<String>) -> Self {
        self.editor_buffer = RefCell::new(editor_buffer);
        self
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn open_browser<S: AsRef<OsStr>>(&self, url: S) {
        self.browser_urls
            .borrow_mut()
            .push(url.as_ref().to_string_lossy().to_string());
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn open_editor<S: AsRef<[u8]>>(&self, _initial_content: S) -> String {
        self.editor_buffer.borrow_mut().remove(0)
    }
}
