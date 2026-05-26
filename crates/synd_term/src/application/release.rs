use super::Application;

impl Application {
    pub(super) fn inform_latest_release(&self) {
        let current_version = env!("CARGO_PKG_VERSION");
        if let Some(new_version) = &self.components.shell.latest_release {
            println!("A new release of synd is available: v{current_version} -> {new_version}");
        }
    }
}
