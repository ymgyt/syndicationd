use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tokio::sync::mpsc;

use crate::crawl::job::CrawlJobTrigger;

/// Worker-facing crawl request whose dispatch order has already been decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DispatchEntry {
    pub(crate) feed_url: FeedUrl,
    pub(crate) trigger: CrawlJobTrigger,
    pub(crate) dispatched_at: DateTime<Utc>,
}

impl DispatchEntry {
    pub(crate) fn new(
        feed_url: FeedUrl,
        trigger: CrawlJobTrigger,
        dispatched_at: DateTime<Utc>,
    ) -> Self {
        Self {
            feed_url,
            trigger,
            dispatched_at,
        }
    }
}

pub(crate) fn dispatch_queue(capacity: usize) -> (DispatchQueueWriter, DispatchQueueReader) {
    let (sender, receiver) = mpsc::channel(capacity);
    (
        DispatchQueueWriter { sender },
        DispatchQueueReader { receiver },
    )
}

/// Sender side of the worker-facing dispatch queue.
#[derive(Debug, Clone)]
pub(crate) struct DispatchQueueWriter {
    sender: mpsc::Sender<DispatchEntry>,
}

impl DispatchQueueWriter {
    pub(crate) fn remaining_capacity(&self) -> usize {
        self.sender.capacity()
    }

    pub(crate) fn push(&self, entry: DispatchEntry) -> Result<(), DispatchQueuePushError> {
        self.sender.try_send(entry).map_err(|err| match err {
            mpsc::error::TrySendError::Full(_) => DispatchQueuePushError::Full,
            mpsc::error::TrySendError::Closed(_) => DispatchQueuePushError::Closed,
        })
    }
}

/// Receiver side of the worker-facing dispatch queue.
#[derive(Debug)]
pub(crate) struct DispatchQueueReader {
    receiver: mpsc::Receiver<DispatchEntry>,
}

impl DispatchQueueReader {
    /// Waits for the next dispatched entry; `None` when all writers dropped.
    pub(crate) async fn recv(&mut self) -> Option<DispatchEntry> {
        self.receiver.recv().await
    }
}

#[derive(Debug)]
pub(crate) enum DispatchQueuePushError {
    Full,
    Closed,
}
