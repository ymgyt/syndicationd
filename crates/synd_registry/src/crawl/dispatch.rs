use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tokio::sync::mpsc;

use crate::crawl::job::CrawlJobTrigger;

/// Scheduler-facing facts available when deciding dispatch output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DispatchContext {
    pub(crate) now: DateTime<Utc>,
    pub(crate) dispatch_queue_len: usize,
    pub(crate) dispatch_queue_remaining_capacity: usize,
}

impl DispatchContext {
    pub(crate) fn new(
        now: DateTime<Utc>,
        dispatch_queue_len: usize,
        dispatch_queue_remaining_capacity: usize,
    ) -> Self {
        Self {
            now,
            dispatch_queue_len,
            dispatch_queue_remaining_capacity,
        }
    }
}

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

/// Ordered dispatch entries returned by one scheduler dispatch decision.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct DispatchBatch {
    entries: Vec<DispatchEntry>,
}

impl DispatchBatch {
    pub(crate) fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn one(entry: DispatchEntry) -> Self {
        Self {
            entries: vec![entry],
        }
    }

    pub(crate) fn into_entries(self) -> Vec<DispatchEntry> {
        self.entries
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
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
    pub(crate) fn len(&self) -> usize {
        self.sender
            .max_capacity()
            .saturating_sub(self.sender.capacity())
    }

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
    pub(crate) fn try_pop(&mut self) -> Option<DispatchEntry> {
        match self.receiver.try_recv() {
            Ok(entry) => Some(entry),
            Err(mpsc::error::TryRecvError::Empty | mpsc::error::TryRecvError::Disconnected) => None,
        }
    }
}

#[derive(Debug)]
pub(crate) enum DispatchQueuePushError {
    Full,
    Closed,
}
