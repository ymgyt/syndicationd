use std::ffi::OsStr;

pub struct Interactor {}

impl Interactor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn open_browser<S: AsRef<OsStr>>(&self, url: S) {
        // try to open input screen in the browser
        open::that(url).ok();
    }

    pub fn open_editor<S: AsRef<[u8]>>(&self, initial_content: S) -> String {
        edit::edit(initial_content).expect("Got user modified input")
    }
}
