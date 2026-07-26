use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use futures_util::Stream;
use synd_client::{Retryability, payload};
use tokio::{sync::mpsc, task::JoinHandle};

use crate::application::FeedApiRef;

use tracing::{debug, warn};

const FEED_EVENT_RECONNECT_DELAY: Duration = Duration::from_secs(2);
const FEED_EVENT_BUFFER_CAPACITY: usize = 1;

/// Long-lived feed event watcher and its event receiver.
pub(in crate::application) enum FeedEventWatcher {
    Stopped,
    Watching {
        rx: mpsc::Receiver<payload::FeedEvent>,
        task: JoinHandle<()>,
    },
    Interrupted,
}

impl FeedEventWatcher {
    pub(super) fn new() -> Self {
        Self::Stopped
    }

    pub(super) fn start(&mut self, feed_api: FeedApiRef) {
        if let Self::Watching { task, .. } = self
            && !task.is_finished()
        {
            return;
        }

        let (tx, rx) = mpsc::channel(FEED_EVENT_BUFFER_CAPACITY);
        let task = tokio::spawn(FeedEventWatchTask::new(feed_api, tx).run());

        *self = Self::Watching { rx, task };
    }

    pub(super) fn stop(&mut self) {
        if let Self::Watching { task, .. } = self {
            task.abort();
        }
        *self = Self::Stopped;
    }

    pub(super) fn restart_if_started(&mut self, feed_api: FeedApiRef) {
        if matches!(self, Self::Stopped) {
            return;
        }
        self.stop();
        self.start(feed_api);
    }
}

/// Runs feed-event watches and applies the terminal's reconnect policy.
struct FeedEventWatchTask {
    feed_api: FeedApiRef,
    events: mpsc::Sender<payload::FeedEvent>,
}

impl FeedEventWatchTask {
    fn new(feed_api: FeedApiRef, events: mpsc::Sender<payload::FeedEvent>) -> Self {
        Self { feed_api, events }
    }

    async fn run(self) {
        loop {
            match self.watch_once().await {
                Ok(()) => {
                    debug!("feed event watcher stopped");
                    return;
                }
                Err(error) if error.retryability() == Retryability::Retryable => {
                    warn!("feed event watcher failed; reconnecting: {error}");
                    tokio::time::sleep(FEED_EVENT_RECONNECT_DELAY).await;
                }
                Err(error) => {
                    warn!("feed event watcher stopped: {error}");
                    return;
                }
            }
        }
    }

    async fn watch_once(&self) -> Result<(), synd_client::SyndApiError> {
        let mut watch = self.feed_api.watch_feed_events().await?;

        loop {
            let event = watch.next_event().await?;
            if self.events.send(event).await.is_err() {
                return Ok(());
            }
        }
    }
}

impl Drop for FeedEventWatcher {
    fn drop(&mut self) {
        if let Self::Watching { task, .. } = self {
            task.abort();
        }
    }
}

impl Stream for FeedEventWatcher {
    type Item = payload::FeedEvent;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        match this {
            Self::Watching { rx, .. } => match rx.poll_recv(cx) {
                Poll::Ready(Some(event)) => Poll::Ready(Some(event)),
                Poll::Ready(None) => {
                    *this = Self::Interrupted;
                    Poll::Pending
                }
                Poll::Pending => Poll::Pending,
            },
            Self::Stopped | Self::Interrupted => Poll::Pending,
        }
    }
}
