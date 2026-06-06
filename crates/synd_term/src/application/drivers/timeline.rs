use std::time::Duration;

use synd_client::payload;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::application::FeedApiRef;

use super::DriverContext;
use tracing::{debug, warn};

const TIMELINE_CHANGE_RECONNECT_DELAY: Duration = Duration::from_secs(2);

/// Running GraphQL timeline change subscription and its event receiver.
pub(in crate::application) struct TimelineChangeSubscription {
    rx: mpsc::UnboundedReceiver<payload::TimelineChangeEvent>,
    task: Option<JoinHandle<()>>,
}

impl TimelineChangeSubscription {
    pub(super) fn new() -> Self {
        let (_tx, rx) = mpsc::unbounded_channel();
        Self { rx, task: None }
    }

    pub(super) fn start(&mut self, feed_api: FeedApiRef) {
        if self.task.is_some() {
            return;
        }

        let (tx, rx) = mpsc::unbounded_channel();
        self.rx = rx;
        self.task = Some(tokio::spawn(async move {
            loop {
                if tx.is_closed() {
                    break;
                }

                match feed_api.run_timeline_changes(tx.clone()).await {
                    Ok(()) => debug!("timeline change subscription stopped"),
                    Err(error) => warn!("timeline change subscription failed: {error}"),
                }

                if tx.is_closed() {
                    break;
                }
                tokio::time::sleep(TIMELINE_CHANGE_RECONNECT_DELAY).await;
            }
        }));
    }

    pub(super) fn stop(&mut self) {
        if let Some(task) = self.task.take() {
            task.abort();
        }
        let (_tx, rx) = mpsc::unbounded_channel();
        self.rx = rx;
    }

    pub(super) fn restart_if_running(&mut self, feed_api: FeedApiRef) -> bool {
        if self.task.is_none() {
            return false;
        }
        self.stop();
        self.start(feed_api);
        true
    }

    pub(in crate::application) async fn recv(&mut self) -> Option<payload::TimelineChangeEvent> {
        self.rx.recv().await
    }
}

pub(super) struct TimelineDriver;

impl TimelineDriver {
    pub(super) fn start_change_subscription(
        cx: &mut DriverContext<'_>,
    ) -> Vec<crate::event::Event> {
        let feed_api = cx.adapters.feed_api.clone();
        cx.timeline_changes.start(feed_api);
        Vec::new()
    }
}
