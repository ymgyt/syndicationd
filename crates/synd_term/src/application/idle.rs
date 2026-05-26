use std::time::Duration;

use tokio::time::Instant;

use super::Application;

impl Application {
    pub(super) fn handle_idle(&mut self) {
        self.clear_idle_timer();

        #[cfg(feature = "integration")]
        {
            tracing::debug!("Quit for idle");
            self.components.shell.quit();
        }
    }

    pub fn clear_idle_timer(&mut self) {
        // https://github.com/tokio-rs/tokio/blob/e53b92a9939565edb33575fff296804279e5e419/tokio/src/time/instant.rs#L62
        self.drivers
            .idle_timer
            .as_mut()
            .reset(Instant::now() + Duration::from_hours(24 * 365 * 30));
    }

    pub fn reset_idle_timer(&mut self) {
        self.drivers
            .idle_timer
            .as_mut()
            .reset(Instant::now() + self.config.idle_timer_interval);
    }
}
