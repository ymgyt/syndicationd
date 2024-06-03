use std::ffi::OsStr;

pub type Interactor = TestInteractor;

pub struct TestInteractor {
    editor_buffer: String,
}

impl TestInteractor {
    pub fn new() -> Self {
        Self {
            editor_buffer: String::new(),
        }
    }

    pub fn with_buffer(mut self, editor_buffer: impl Into<String>) -> Self {
        self.editor_buffer = editor_buffer.into();
        self
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn open_browser<S: AsRef<OsStr>>(&self, _url: S) {
        // do nothing
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn open_editor<S: AsRef<[u8]>>(&self, _initial_content: S) -> String {
        self.editor_buffer.clone()
    }
}
