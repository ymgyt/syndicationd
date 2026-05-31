use super::Application;
#[cfg(feature = "integration")]
use tracing::debug;

impl Application {
    pub(super) fn handle_idle(&mut self) {
        self.clear_idle_timer();

        #[cfg(feature = "integration")]
        {
            debug!("Quit for idle");
            self.components.shell.quit();
        }
    }

    pub fn clear_idle_timer(&mut self) {
        self.drivers.clear_idle_timer();
    }

    pub fn reset_idle_timer(&mut self) {
        self.drivers
            .reset_idle_timer(self.config.idle_timer_interval);
    }
}
