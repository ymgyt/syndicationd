use crossterm::event::Event as CrosstermEvent;
use futures_util::Stream;
use ratatui::buffer::Buffer;

use crate::operation::Operation;

use super::Application;

impl Application {
    pub fn buffer(&self) -> &Buffer {
        self.terminal.buffer()
    }

    pub fn bootstrap_for_test(&mut self) {
        let operations = self
            .compositor
            .apply_event(&crate::event::Event::ApiAccessReady);
        self.drivers.dispatch(operations, &mut self.terminal);
    }

    pub async fn wait_until_jobs_completed<S>(&mut self, input: &mut S)
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        while !self.compositor.should_quit() {
            self.render().expect("render test terminal");
            tokio::select! {
                biased;
                result = self.process_next(input) => result.expect("process test event"),
                () = std::future::ready(()), if self.jobs_completed() => return,
            }
        }
        assert!(self.jobs_completed(), "TUI exited with unfinished requests");
    }

    fn jobs_completed(&self) -> bool {
        self.drivers.request_jobs_is_empty()
            && !self.compositor.is_busy()
            && self.compositor.components.feeds.timeline_is_settled()
    }

    pub async fn run_until<S>(
        &mut self,
        input: &mut S,
        shown: impl Fn(&Buffer) -> bool,
    ) -> anyhow::Result<()>
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        loop {
            self.render()?;
            if shown(self.buffer()) {
                return Ok(());
            }
            self.process_next(input).await?;
            anyhow::ensure!(
                !self.compositor.should_quit(),
                "TUI exited before the expected screen"
            );
        }
    }

    pub async fn reload_cache(&mut self) -> anyhow::Result<()> {
        let credential = self.drivers.restore_credential().await?;
        self.drivers.dispatch(
            Operation::SetCredential { credential }.into(),
            &mut self.terminal,
        );
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
