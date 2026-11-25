use futures_util::Stream;
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use tuiframework::crossterm::event::Event as CrosstermEvent;

use crate::operation::Operation;

use super::Application;

/// Runs the synd screens against a terminal buffer with controlled input.
pub struct TestApplication {
    app: Application,
    terminal: Terminal<TestBackend>,
}
impl Application {
    pub fn with_test_terminal(self, width: u16, height: u16) -> TestApplication {
        TestApplication {
            app: self,
            terminal: Terminal::new(TestBackend::new(width, height)).unwrap(),
        }
    }
}
impl TestApplication {
    pub fn buffer(&self) -> &Buffer {
        self.terminal.backend().buffer()
    }

    pub async fn wait_until_jobs_completed<S>(&mut self, input: &mut S)
    where
        S: Stream<Item = std::io::Result<CrosstermEvent>> + Unpin,
    {
        while !self.app.tui.is_finished() {
            self.app
                .tui
                .render(&mut self.terminal)
                .expect("render test terminal");
            tokio::select! {
                biased;
                result = self.app.tui.step(input) => result.expect("process test event"),
                () = std::future::ready(()), if self.jobs_completed() => return,
            }
        }
        assert!(self.jobs_completed(), "TUI exited with unfinished requests");
    }

    fn jobs_completed(&self) -> bool {
        !self.app.tui.has_pending_work()
            && !tuiframework::Components::is_busy(self.app.tui.components())
            && self.app.tui.components().feeds.timeline_is_settled()
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
            self.app.tui.render(&mut self.terminal)?;
            if shown(self.buffer()) {
                return Ok(());
            }
            self.app.tui.step(input).await?;
            anyhow::ensure!(
                !self.app.tui.is_finished(),
                "TUI exited before the expected screen"
            );
        }
    }

    pub async fn reload_cache(&mut self) -> anyhow::Result<()> {
        let credential = self.app.tui.driver().restore_credential().await?;
        self.app
            .tui
            .dispatch(Operation::SetCredential { credential });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{Application, Cache, Config, outbound::feed::MockFeedApi},
        config::Categories,
        interact::mock::MockInteractor,
        ui::theme::Theme,
    };
    use futures_util::{FutureExt as _, stream};

    #[tokio::test]
    async fn eof_finishes_the_loop_without_waiting_for_a_driver_event() {
        let cache = tempfile::tempdir().unwrap();
        let mut app = Application::builder()
            .feed_api(MockFeedApi::new([]))
            .categories(Categories::default_toml())
            .config(Config::default())
            .cache(Cache::new(cache.path()))
            .theme(Theme::default())
            .interactor(Box::new(MockInteractor::new()))
            .build()
            .with_test_terminal(80, 24);
        app.app
            .tui
            .run_with(&mut app.terminal, &mut stream::empty())
            .now_or_never()
            .expect("EOF must be ready")
            .unwrap();
    }
}
