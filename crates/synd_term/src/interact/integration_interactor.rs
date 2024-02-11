use std::ffi::OsStr;

pub type Interactor = TestInteractor;

pub struct TestInteractor;

impl TestInteractor {
    pub fn new() -> Self {
        Self {}
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn open_browser<S: AsRef<OsStr>>(&self, _url: S) {
        // do nothing
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn open_editor<S: AsRef<[u8]>>(&self, _initial_content: S) -> String {
        String::new()
    }
}
