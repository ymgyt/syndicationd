#[cfg(feature = "integration")]
use crossterm::event::Event as CrosstermEvent;
#[cfg(feature = "integration")]
use futures_util::Stream;

#[cfg(feature = "integration")]
use crate::operation::Operation;

#[cfg(feature = "integration")]
use super::Application;

#[cfg(feature = "integration")]
impl Application {
    pub fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.drivers.buffer()
    }

    pub fn bootstrap_for_test(&mut self) {
        let operations = self.bootstrap().into();
        self.drivers.dispatch(operations);
        self.reset_idle_timer();
    }

    pub async fn wait_until_jobs_completed<S>(&mut self, input: &mut S)
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        loop {
            self.event_loop_until_idle(input)
                .await
                .expect("integration event loop must remain usable");
            self.reset_idle_timer();

            if self.drivers.request_jobs_is_empty()
                && self.components.shell.in_flight.is_empty()
                && self.components.feeds.timeline_is_settled()
            {
                break;
            }
        }
    }

    pub async fn event_loop_until_idle<S>(&mut self, input: &mut S) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.process_until_quit(input).await
    }

    pub async fn reload_cache(&mut self) -> anyhow::Result<()> {
        let credential = self.drivers.restore_credential().await?;
        self.drivers
            .dispatch(Operation::SetCredential { credential }.into());
        Ok(())
    }
}
