use std::{future, time::Duration};

use synd_client::payload;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::application::FeedApiRef;

use tracing::{debug, warn};

const FEED_EVENT_RECONNECT_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub(in crate::application) enum FeedEventMessage {
    Event(payload::FeedEvent),
    Interrupted,
}

/// Running GraphQL feed event subscription and its event receiver.
pub(in crate::application) enum FeedEventSubscription {
    Stopped,
    Running {
        rx: mpsc::UnboundedReceiver<FeedEventMessage>,
        task: JoinHandle<()>,
    },
}

impl FeedEventSubscription {
    pub(super) fn new() -> Self {
        Self::Stopped
    }

    pub(super) fn start(&mut self, feed_api: FeedApiRef) {
        if matches!(self, Self::Running { .. }) {
            return;
        }

        let (tx, rx) = mpsc::unbounded_channel();
        let task = tokio::spawn(async move {
            let mut interruption_notified = false;
            loop {
                if tx.is_closed() {
                    break;
                }

                let (event_tx, mut event_rx) = mpsc::unbounded_channel();
                let mut run = feed_api.run_feed_events(event_tx);
                loop {
                    tokio::select! {
                        biased;

                        Some(event) = event_rx.recv() => {
                            interruption_notified = false;
                            if tx.send(FeedEventMessage::Event(event)).is_err() {
                                return;
                            }
                        }
                        result = &mut run => {
                            match result {
                                Ok(()) => debug!("feed event subscription stopped"),
                                Err(error) => warn!("feed event subscription failed: {error}"),
                            }
                            if !interruption_notified
                                && tx.send(FeedEventMessage::Interrupted).is_ok()
                            {
                                interruption_notified = true;
                            }
                            break;
                        }
                    }
                }

                if tx.is_closed() {
                    break;
                }
                tokio::time::sleep(FEED_EVENT_RECONNECT_DELAY).await;
            }
        });

        *self = Self::Running { rx, task };
    }

    pub(super) fn stop(&mut self) {
        if let Self::Running { task, .. } = std::mem::replace(self, Self::Stopped) {
            task.abort();
        }
    }

    pub(super) fn restart_if_running(&mut self, feed_api: FeedApiRef) -> bool {
        if matches!(self, Self::Stopped) {
            return false;
        }
        self.stop();
        self.start(feed_api);
        true
    }

    pub(in crate::application) async fn recv(&mut self) -> Option<FeedEventMessage> {
        match self {
            Self::Running { rx, .. } => rx.recv().await,
            Self::Stopped => future::pending().await,
        }
    }
}
