pub struct Interactor {}

impl Interactor {
    pub fn new() -> Self {
        Self {}
    }

    pub fn open_browser(&self, url: String) {
        // try to open input screen in the browser
        open::that(url).ok();
    }
}
