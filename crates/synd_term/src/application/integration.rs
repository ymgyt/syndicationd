use super::{Application, Drivers, Effects, Ui};
use crate::integration::{UnboundedSenderWrapper, event_stream};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};
use std::io;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tuiframework::crossterm::event::Event as CrosstermEvent;
use tuiframework::event_loop::Started;
use tuiframework::{EventLoop, Ui as _};

use crate::operation::Operation;

type TestEventLoop = EventLoop<
    Started<'static, Ui, Drivers>,
    Terminal<TestBackend>,
    UnboundedReceiverStream<io::Result<CrosstermEvent>>,
>;

/// Runs the synd screens against a terminal buffer with controlled input.
pub struct TestApplication {
    events: TestEventLoop,
    input: UnboundedSenderWrapper,
}
impl Application {
    pub fn with_test_terminal(self, width: u16, height: u16) -> TestApplication {
        let (input, receiver) = event_stream();
        let terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let events = self.event_loop(terminal, receiver).start();
        TestApplication { events, input }
    }
}
impl TestApplication {
    pub fn input(&self) -> UnboundedSenderWrapper {
        self.input.clone()
    }

    pub fn buffer(&self) -> &Buffer {
        self.events.terminal().backend().buffer()
    }

    pub async fn wait_until_jobs_completed(&mut self) {
        while !self.events.should_quit() {
            self.events.render().expect("render test terminal");
            tokio::select! {
                biased;
                result = self.events.process_next() => result.expect("process test event"),
                () = std::future::ready(()), if self.jobs_completed() => return,
            }
        }
        assert!(self.jobs_completed(), "TUI exited with unfinished requests");
    }

    fn jobs_completed(&self) -> bool {
        !self.events.has_pending_work()
            && !self.events.ui().is_busy()
            && self.events.ui().feeds.timeline_is_settled()
    }

    pub async fn run_until(&mut self, shown: impl Fn(&Buffer) -> bool) -> anyhow::Result<()> {
        while !self.events.should_quit() {
            self.events.render()?;
            if shown(self.buffer()) {
                return Ok(());
            }
            self.events.process_next().await?;
        }
        anyhow::bail!("TUI exited before the expected screen")
    }

    pub async fn reload_cache(&mut self) -> anyhow::Result<()> {
        if !self.events.should_quit() {
            let credential = self.events.driver().restore_credential().await?;
            self.events
                .apply_effects(Effects::NONE.operation(Operation::SetCredential { credential }));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use futures_util::{FutureExt as _, stream};
    use ratatui::{Terminal, backend::TestBackend};

    use crate::{
        application::{
            Application, Cache, ComponentId, Config, Features, outbound::feed::MockFeedApi,
        },
        client::gh::FetchNotificationInclude,
        command::{Command, GhCommand, GhNotificationFilterOption},
        config::Categories,
        interact::mock::MockInteractor,
        ui::{theme::Theme, widgets::gh_notifications::GhNotificationFilterOptions},
    };

    #[tokio::test]
    async fn eof_persists_the_applied_github_filter_without_waiting_for_a_request() {
        let dir = tempfile::tempdir().unwrap();
        let cache = Cache::new(dir.path());
        let stored = GhNotificationFilterOptions {
            include: FetchNotificationInclude::All,
            ..Default::default()
        };
        cache
            .persist_gh_notification_filter_options(&stored)
            .unwrap();
        let app = Application::builder()
            .feed_api(MockFeedApi::new([]))
            .categories(Categories::default_toml())
            .config(Config {
                features: Features {
                    enable_gh_notification: true,
                },
                ..Config::default()
            })
            .authentication_required(true)
            .cache(Cache::new(dir.path()))
            .theme(Theme::default())
            .interactor(Box::new(MockInteractor::new()))
            .build();
        let terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let mut running = app.event_loop(terminal, stream::empty()).start();
        assert_eq!(running.ui().gh.notifications.filter_options(), &stored);
        running.apply_effects(super::Effects::NONE.send_all(
            ComponentId::GhNotifications,
            [
                Command::Gh(GhCommand::ToggleNotificationFilter(
                    GhNotificationFilterOption::UnreadOnly,
                )),
                Command::Gh(GhCommand::ApplyNotificationFilter),
            ],
        ));
        running
            .run()
            .now_or_never()
            .expect("EOF must be ready")
            .unwrap();
        assert_eq!(
            cache.load_gh_notification_filter_options().unwrap(),
            GhNotificationFilterOptions::default()
        );
    }
}
