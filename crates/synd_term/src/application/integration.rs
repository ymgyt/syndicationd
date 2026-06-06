#[cfg(feature = "integration")]
use super::Application;
#[cfg(feature = "integration")]
use crossterm::event::Event as CrosstermEvent;
#[cfg(feature = "integration")]
use futures_util::Stream;

#[cfg(feature = "integration")]
impl<Term, Sess> Application<Term, Sess> {
    pub fn buffer(&self) -> &ratatui::buffer::Buffer {
        self.drivers.buffer()
    }

    pub async fn wait_until_jobs_completed<S>(&mut self, input: &mut S)
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        loop {
            let _ = self.event_loop_until_idle(input).await;
            self.reset_idle_timer();

            // Long-lived background jobs such as credential refresh and periodic
            // feed-view sync are intentionally not drained here. Short feed
            // refresh polls and timeline reload debounce are component state, so
            // wait for those explicit states instead of the whole background queue.
            if self.drivers.foreground_jobs_is_empty()
                && !self.components.feeds.has_pending_short_background_work()
            {
                break;
            }
        }
    }

    pub async fn reload_cache(&mut self) -> anyhow::Result<()> {
        match self.restore_credential().await {
            Ok(cred) => self.handle_restored_credential(cred),
            Err(err) => return Err(err.into()),
        }
        Ok(())
    }
}
