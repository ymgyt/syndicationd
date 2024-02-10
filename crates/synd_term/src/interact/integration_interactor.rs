pub type Interactor = TestInteractor;

pub struct TestInteractor;

impl TestInteractor {
    pub fn new() -> Self {
        Self {}
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn open_browser(&self, _url: String) {
        // do nothing
    }

    #[allow(clippy::unused_self, clippy::needless_pass_by_value)]
    pub fn open_editor<S: AsRef<[u8]>>(&self, _initial_content: S) -> String {
        String::new()
    }
}
