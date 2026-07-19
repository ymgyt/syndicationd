use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use chrono::{DateTime, Utc};
use synd_feed::types::FeedUrl;
use tokio::sync::mpsc;

use crate::crawl::job::CrawlJobTrigger;

/// In-process guard against dispatching a feed that is already being crawled.
///
/// Not persisted: a crash loses only the guard, and the next scheduler pass
/// re-derives due feeds from durable state (worst case: one duplicate crawl).
#[derive(Debug, Clone, Default)]
pub(crate) struct InflightCrawls {
    inner: Arc<Mutex<HashSet<FeedUrl>>>,
}

impl InflightCrawls {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn contains(&self, feed_url: &FeedUrl) -> bool {
        self.inner
            .lock()
            .expect("inflight lock is never poisoned")
            .contains(feed_url)
    }

    /// Claims the feed for one crawl. `None` while a claim is outstanding.
    pub(crate) fn try_claim(&self, feed_url: &FeedUrl) -> Option<InflightGuard> {
        let mut set = self.inner.lock().expect("inflight lock is never poisoned");
        set.insert(feed_url.clone()).then(|| InflightGuard {
            set: Arc::clone(&self.inner),
            feed_url: feed_url.clone(),
        })
    }
}

/// Releases the inflight claim when the crawl ends, however it ends.
#[derive(Debug)]
pub(crate) struct InflightGuard {
    set: Arc<Mutex<HashSet<FeedUrl>>>,
    feed_url: FeedUrl,
}

impl Drop for InflightGuard {
    fn drop(&mut self) {
        self.set
            .lock()
            .expect("inflight lock is never poisoned")
            .remove(&self.feed_url);
    }
}

/// Worker-facing crawl request whose dispatch order has already been decided.
///
/// Carries the inflight claim: dropping the entry (or finishing the crawl)
/// releases the claim.
#[derive(Debug)]
pub(crate) struct DispatchEntry {
    pub(crate) feed_url: FeedUrl,
    pub(crate) trigger: CrawlJobTrigger,
    pub(crate) dispatched_at: DateTime<Utc>,
    pub(crate) inflight: InflightGuard,
}

impl DispatchEntry {
    pub(crate) fn new(
        feed_url: FeedUrl,
        trigger: CrawlJobTrigger,
        dispatched_at: DateTime<Utc>,
        inflight: InflightGuard,
    ) -> Self {
        Self {
            feed_url,
            trigger,
            dispatched_at,
            inflight,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflight_claim_is_exclusive_until_dropped() {
        let inflight = InflightCrawls::new();
        let feed_url = FeedUrl::parse("https://example.com/feed.xml").unwrap();

        let guard = inflight.try_claim(&feed_url).expect("first claim");
        assert!(inflight.try_claim(&feed_url).is_none());

        drop(guard);
        assert!(inflight.try_claim(&feed_url).is_some());
    }
}
