use crossterm::event::Event as CrosstermEvent;
use futures_util::Stream;
use ratatui::buffer::Buffer;

use crate::operation::Operation;

use super::Application;

impl Application {
    pub fn buffer(&self) -> &Buffer {
        self.drivers.buffer()
    }

    pub fn bootstrap_for_test(&mut self) {
        let operations = self.ui.bootstrap();
        self.drivers.dispatch(operations);
    }

    pub async fn wait_until_jobs_completed<S>(&mut self, input: &mut S)
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.render().expect("render test terminal");
        loop {
            let settled = self.drivers.request_jobs_is_empty()
                && !self.ui.is_busy()
                && self.ui.components.feeds.timeline_is_settled();
            tokio::select! {
                biased;
                result = self.process_next(input) => {
                    if result.expect("process test event").is_break() {
                        assert!(settled, "TUI exited with unfinished requests");
                        break;
                    }
                }
                () = std::future::ready(()), if settled => break,
            }
        }
    }

    pub async fn run_until<S>(
        &mut self,
        input: &mut S,
        shown: impl Fn(&Buffer) -> bool,
    ) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        self.render()?;
        while !shown(self.buffer()) {
            anyhow::ensure!(
                self.process_next(input).await?.is_continue(),
                "TUI exited before the expected screen"
            );
        }
        Ok(())
    }

    pub async fn reload_cache(&mut self) -> anyhow::Result<()> {
        let credential = self.drivers.restore_credential().await?;
        self.drivers
            .dispatch(Operation::SetCredential { credential }.into());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{Application, Cache, Config, outbound::feed::MockFeedApi},
        config::Categories,
        integration::new_test_terminal,
        interact::mock::MockInteractor,
        ui::theme::Theme,
    };
    use futures_util::{FutureExt as _, stream};

    #[tokio::test]
    async fn eof_finishes_the_loop_without_waiting_for_a_driver_event() {
        let cache = tempfile::tempdir().unwrap();
        let mut app = Application::builder()
            .terminal(new_test_terminal(80, 24))
            .feed_api(MockFeedApi::new([]))
            .categories(Categories::default_toml())
            .config(Config::default())
            .cache(Cache::new(cache.path()))
            .theme(Theme::default())
            .interactor(Box::new(MockInteractor::new()))
            .build();
        app.event_loop(&mut stream::empty())
            .now_or_never()
            .expect("EOF must be ready")
            .unwrap();
    }
}
